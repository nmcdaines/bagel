use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn test_app() -> Router {
    let pool = bagel::db::connect("sqlite::memory:").await.unwrap();
    bagel::app("does-not-exist", pool)
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let req = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(body) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}

async fn create_note(app: &Router, title: &str, body: &str) -> Value {
    let (status, note) = send(
        app,
        "POST",
        "/api/notes",
        Some(json!({ "title": title, "body": body })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    note
}

#[tokio::test]
async fn create_returns_note() {
    let app = test_app().await;
    let note = create_note(&app, "  Groceries ", "eggs").await;
    assert!(note["id"].as_i64().unwrap() > 0);
    assert_eq!(note["title"], "Groceries");
    assert_eq!(note["body"], "eggs");
    assert!(note["created_at"].is_string());
    assert_eq!(note["created_at"], note["updated_at"]);
}

#[tokio::test]
async fn create_defaults_body_to_empty() {
    let app = test_app().await;
    let (status, note) = send(&app, "POST", "/api/notes", Some(json!({ "title": "t" }))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(note["body"], "");
}

#[tokio::test]
async fn list_returns_newest_first() {
    let app = test_app().await;
    let (status, notes) = send(&app, "GET", "/api/notes", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(notes, json!([]));

    let first = create_note(&app, "first", "").await;
    let second = create_note(&app, "second", "").await;

    let (status, notes) = send(&app, "GET", "/api/notes", None).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<_> = notes
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].clone())
        .collect();
    assert_eq!(ids, vec![second["id"].clone(), first["id"].clone()]);
}

#[tokio::test]
async fn get_returns_note() {
    let app = test_app().await;
    let note = create_note(&app, "hello", "world").await;
    let (status, fetched) = send(&app, "GET", &format!("/api/notes/{}", note["id"]), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched, note);
}

#[tokio::test]
async fn update_changes_note() {
    let app = test_app().await;
    let note = create_note(&app, "old", "old body").await;
    let uri = format!("/api/notes/{}", note["id"]);

    let (status, updated) = send(
        &app,
        "PUT",
        &uri,
        Some(json!({ "title": "new", "body": "new body" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["id"], note["id"]);
    assert_eq!(updated["title"], "new");
    assert_eq!(updated["body"], "new body");
    assert_eq!(updated["created_at"], note["created_at"]);

    let (_, fetched) = send(&app, "GET", &uri, None).await;
    assert_eq!(fetched, updated);
}

#[tokio::test]
async fn delete_removes_note() {
    let app = test_app().await;
    let note = create_note(&app, "bye", "").await;
    let uri = format!("/api/notes/{}", note["id"]);

    let (status, body) = send(&app, "DELETE", &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(body, Value::Null);

    let (status, _) = send(&app, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_note_is_404() {
    let app = test_app().await;
    let (status, body) = send(&app, "GET", "/api/notes/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "note not found");

    let (status, _) = send(
        &app,
        "PUT",
        "/api/notes/999",
        Some(json!({ "title": "x", "body": "" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(&app, "DELETE", "/api/notes/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn empty_title_is_rejected() {
    let app = test_app().await;
    for title in ["", "   "] {
        let (status, body) = send(
            &app,
            "POST",
            "/api/notes",
            Some(json!({ "title": title, "body": "b" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "title must not be empty");
    }

    let note = create_note(&app, "keep", "").await;
    let uri = format!("/api/notes/{}", note["id"]);
    let (status, _) = send(&app, "PUT", &uri, Some(json!({ "title": " ", "body": "" }))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (_, fetched) = send(&app, "GET", &uri, None).await;
    assert_eq!(fetched["title"], "keep");

    let (_, notes) = send(&app, "GET", "/api/notes", None).await;
    assert_eq!(notes.as_array().unwrap().len(), 1);
}

/// Sends a raw body and returns the status, content type and parsed JSON error body.
async fn send_raw(
    app: &Router,
    method: &str,
    uri: &str,
    content_type: Option<&str>,
    body: impl Into<Body>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(content_type) = content_type {
        req = req.header(header::CONTENT_TYPE, content_type);
    }
    let res = app
        .clone()
        .oneshot(req.body(body.into()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    assert_eq!(
        res.headers()[header::CONTENT_TYPE],
        "application/json",
        "{method} {uri} -> {status} is not JSON"
    );
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// Asserts that both POST /api/notes and PUT /api/notes/{id} reject `body` with `status` and a
/// JSON `error` message containing `needle`.
async fn assert_rejected(
    app: &Router,
    content_type: Option<&str>,
    body: &str,
    status: StatusCode,
    needle: &str,
) {
    let note = create_note(app, "existing", "").await;
    let note_uri = format!("/api/notes/{}", note["id"]);
    for (method, uri) in [("POST", "/api/notes"), ("PUT", note_uri.as_str())] {
        let (actual, err) = send_raw(app, method, uri, content_type, body.to_owned()).await;
        assert_eq!(actual, status, "{method} {uri} with {body:?}");
        let message = err["error"].as_str().unwrap();
        assert!(
            message.contains(needle),
            "{method} {uri}: {message:?} does not contain {needle:?}"
        );
    }
}

const JSON: Option<&str> = Some("application/json");

#[tokio::test]
async fn malformed_json_is_400() {
    let app = test_app().await;
    assert_rejected(
        &app,
        JSON,
        r#"{"title": "#,
        StatusCode::BAD_REQUEST,
        "Failed to parse the request body as JSON",
    )
    .await;
}

#[tokio::test]
async fn missing_title_is_422() {
    let app = test_app().await;
    assert_rejected(
        &app,
        JSON,
        "{}",
        StatusCode::UNPROCESSABLE_ENTITY,
        "missing field `title`",
    )
    .await;
}

#[tokio::test]
async fn null_title_is_422() {
    let app = test_app().await;
    assert_rejected(
        &app,
        JSON,
        r#"{"title": null}"#,
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid type: null",
    )
    .await;
}

#[tokio::test]
async fn null_body_is_422() {
    let app = test_app().await;
    assert_rejected(
        &app,
        JSON,
        r#"{"title": "x", "body": null}"#,
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid type: null",
    )
    .await;
}

#[tokio::test]
async fn missing_content_type_is_415() {
    let app = test_app().await;
    let body = r#"{"title": "x"}"#;
    assert_rejected(
        &app,
        None,
        body,
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Content-Type",
    )
    .await;
    assert_rejected(
        &app,
        Some("text/plain"),
        body,
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Content-Type",
    )
    .await;
}

#[tokio::test]
async fn oversized_body_is_413() {
    let app = test_app().await;
    let body = format!(
        r#"{{"title": "x", "body": "{}"}}"#,
        "a".repeat(3 * 1024 * 1024)
    );
    assert_rejected(
        &app,
        JSON,
        &body,
        StatusCode::PAYLOAD_TOO_LARGE,
        "length limit",
    )
    .await;
}

#[tokio::test]
async fn invalid_path_id_is_400() {
    let app = test_app().await;
    for method in ["GET", "PUT", "DELETE"] {
        let (status, err) =
            send_raw(&app, method, "/api/notes/abc", JSON, r#"{"title": "x"}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method}");
        let message = err["error"].as_str().unwrap();
        assert!(message.contains("abc"), "{method}: {message:?}");
    }
}
