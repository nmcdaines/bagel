use std::path::Path;

use axum::{Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use utoipa::{
    ToSchema,
    openapi::{InfoBuilder, OpenApiBuilder},
};
use utoipa_axum::{router::OpenApiRouter, routes};

pub mod db;
pub mod notes;

pub fn app(static_dir: impl AsRef<Path>, pool: SqlitePool) -> Router {
    let static_dir = static_dir.as_ref();
    // Unknown paths fall back to index.html so client-side routes work on reload.
    let spa = ServeDir::new(static_dir).fallback(ServeFile::new(static_dir.join("index.html")));

    let (api, _) = api().split_for_parts();
    api.with_state(pool)
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
}

/// The OpenAPI document describing `/api`. This is the contract with the frontend: it is
/// committed as `openapi.json` and the TypeScript client types are generated from it.
pub fn openapi() -> utoipa::openapi::OpenApi {
    api().split_for_parts().1
}

fn api() -> OpenApiRouter<SqlitePool> {
    // Handlers must be registered via `routes!` to appear in the OpenAPI document.
    let api = OpenApiRouter::new()
        .routes(routes!(health))
        .merge(notes::router());
    let info = InfoBuilder::new()
        .title("bagel")
        .version(env!("CARGO_PKG_VERSION"))
        .build();
    OpenApiRouter::with_openapi(OpenApiBuilder::new().info(info).build()).nest("/api", api)
}

#[derive(Serialize, ToSchema)]
struct Health {
    status: &'static str,
    version: &'static str,
}

#[utoipa::path(get, path = "/health", responses((status = OK, description = "Service is up", body = Health)))]
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
        let pool = db::connect("sqlite::memory:").await.unwrap();
        let res = app("does-not-exist", pool)
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

    #[test]
    fn openapi_json_is_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");
        let committed = std::fs::read_to_string(path).unwrap_or_default();
        let current = openapi().to_pretty_json().unwrap() + "\n";
        assert!(
            committed == current,
            "openapi.json is stale; run `npm run gen:api` in web/ and commit the result"
        );
    }
}
