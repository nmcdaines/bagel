//! Error handling shared by the API handlers: every error response is a JSON [`ErrorBody`].

use axum::{
    Json,
    extract::{
        FromRequest, FromRequestParts, Path, Query, Request,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::{Serialize, de::DeserializeOwned};
use utoipa::ToSchema;

/// `Json` extractor whose rejections are reported as a JSON [`ErrorBody`]. It keeps axum's
/// status codes: 400 invalid JSON syntax, 413 body too large, 415 missing/wrong content type,
/// 422 JSON that doesn't match the target type (missing field, `null`, wrong type).
pub struct ApiJson<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequest<S> for ApiJson<T> {
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        Ok(Self(value))
    }
}

/// `Query` extractor whose rejections are reported as a JSON [`ErrorBody`] (400 for an
/// unparseable parameter such as `?project_id=abc`).
pub struct ApiQuery<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequestParts<S> for ApiQuery<T> {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state).await?;
        Ok(Self(value))
    }
}

/// `Path` extractor whose rejections are reported as a JSON [`ErrorBody`] (400 for an
/// unparseable parameter such as `/api/notes/abc`).
pub struct ApiPath<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned + Send> FromRequestParts<S> for ApiPath<T> {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state).await?;
        Ok(Self(value))
    }
}

#[derive(Debug)]
pub enum ApiError {
    /// No such resource; carries its name, e.g. `"note"`.
    NotFound(&'static str),
    Validation(&'static str),
    /// An extractor rejected the request; carries axum's status and message.
    Rejected(StatusCode, String),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::Rejected(rejection.status(), rejection.body_text())
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        Self::Rejected(rejection.status(), rejection.body_text())
    }
}

impl From<PathRejection> for ApiError {
    fn from(rejection: PathRejection) -> Self {
        Self::Rejected(rejection.status(), rejection.body_text())
    }
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::NotFound(resource) => (StatusCode::NOT_FOUND, format!("{resource} not found")),
            Self::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.to_owned()),
            Self::Rejected(status, msg) => (status, msg),
            Self::Database(err) => {
                tracing::error!(error = %err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
        };
        (status, Json(ErrorBody { error })).into_response()
    }
}
