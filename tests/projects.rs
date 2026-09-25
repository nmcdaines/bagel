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

async fn create(app: &Router, uri: &str, body: Value) -> Value {
    let (status, created) = send(app, "POST", uri, Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    created
}

fn ids(list: &Value) -> Vec<Value> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].clone())
        .collect()
}

#[tokio::test]
async fn project_crud() {
    let app = test_app().await;
    let project = create(&app, "/api/projects", json!({ "name": "  Bagel " })).await;
    assert_eq!(project["name"], "Bagel");
    assert_eq!(project["description"], "");
    let uri = format!("/api/projects/{}", project["id"]);

    let (status, fetched) = send(&app, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched, project);

    let (status, updated) = send(
        &app,
        "PUT",
        &uri,
        Some(json!({ "name": "Bagel 2", "description": "Everything" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["name"], "Bagel 2");
    assert_eq!(updated["description"], "Everything");
    assert_eq!(updated["created_at"], project["created_at"]);

    let (status, list) = send(&app, "GET", "/api/projects", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list, json!([updated]));

    let (status, _) = send(&app, "DELETE", &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body) = send(&app, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "project not found");
}

#[tokio::test]
async fn missing_project_is_404() {
    let app = test_app().await;
    let body = Some(json!({ "name": "x" }));
    assert_eq!(
        send(&app, "PUT", "/api/projects/999", body).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "DELETE", "/api/projects/999", None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn empty_project_name_is_rejected() {
    let app = test_app().await;
    let (status, body) = send(&app, "POST", "/api/projects", Some(json!({ "name": " " }))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "name must not be empty");
}

#[tokio::test]
async fn notes_are_optional_in_a_project_and_filterable() {
    let app = test_app().await;
    let bagel = create(&app, "/api/projects", json!({ "name": "Bagel" })).await;
    let other = create(&app, "/api/projects", json!({ "name": "Other" })).await;

    let loose = create(&app, "/api/notes", json!({ "title": "loose" })).await;
    assert_eq!(loose["project_id"], Value::Null);
    let filed = create(
        &app,
        "/api/notes",
        json!({ "title": "filed", "project_id": bagel["id"] }),
    )
    .await;
    assert_eq!(filed["project_id"], bagel["id"]);

    let (_, all) = send(&app, "GET", "/api/notes", None).await;
    assert_eq!(ids(&all), vec![filed["id"].clone(), loose["id"].clone()]);

    let bagel_notes = format!("/api/notes?project_id={}", bagel["id"]);
    let (status, list) = send(&app, "GET", &bagel_notes, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ids(&list), vec![filed["id"].clone()]);

    // Move the loose note into the other project.
    let (status, moved) = send(
        &app,
        "PUT",
        &format!("/api/notes/{}", loose["id"]),
        Some(json!({ "title": "loose", "project_id": other["id"] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(moved["project_id"], other["id"]);
    let (_, list) = send(
        &app,
        "GET",
        &format!("/api/notes?project_id={}", other["id"]),
        None,
    )
    .await;
    assert_eq!(ids(&list), vec![loose["id"].clone()]);
}

#[tokio::test]
async fn note_with_unknown_project_is_rejected() {
    let app = test_app().await;
    let (status, body) = send(
        &app,
        "POST",
        "/api/notes",
        Some(json!({ "title": "x", "project_id": 999 })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "project does not exist");

    let note = create(&app, "/api/notes", json!({ "title": "x" })).await;
    let (status, body) = send(
        &app,
        "PUT",
        &format!("/api/notes/{}", note["id"]),
        Some(json!({ "title": "x", "project_id": 999 })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "project does not exist");
}

#[tokio::test]
async fn deleting_a_project_keeps_its_notes() {
    let app = test_app().await;
    let project = create(&app, "/api/projects", json!({ "name": "Bagel" })).await;
    let note = create(
        &app,
        "/api/notes",
        json!({ "title": "keep me", "project_id": project["id"] }),
    )
    .await;

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/projects/{}", project["id"]),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, fetched) = send(&app, "GET", &format!("/api/notes/{}", note["id"]), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["project_id"], Value::Null);
}

#[tokio::test]
async fn invalid_project_filter_is_400() {
    let app = test_app().await;
    let (status, body) = send(&app, "GET", "/api/notes?project_id=abc", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"].as_str().unwrap().contains("project_id"),
        "{body}"
    );
}
