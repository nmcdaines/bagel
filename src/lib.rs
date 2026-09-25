use std::path::Path;

use axum::{Json, Router, routing::get};
use serde::Serialize;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

pub fn app(static_dir: impl AsRef<Path>) -> Router {
    let static_dir = static_dir.as_ref();
    // Unknown paths fall back to index.html so client-side routes work on reload.
    let spa = ServeDir::new(static_dir).fallback(ServeFile::new(static_dir.join("index.html")));

    Router::new()
        .nest("/api", api())
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
}

fn api() -> Router {
    Router::new().route("/health", get(health))
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_ok() {
        let res = app("does-not-exist")
            .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        assert!(
            std::str::from_utf8(&body)
                .unwrap()
                .contains(r#""status":"ok""#)
        );
    }
}
