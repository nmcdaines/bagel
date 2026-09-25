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
