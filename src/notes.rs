use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<SqlitePool> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(fetch, update, remove))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NoteInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
}

impl NoteInput {
    fn validate(self) -> Result<Self, ApiError> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err(ApiError::Validation("title must not be empty"));
        }
        Ok(Self {
            title: title.to_owned(),
            body: self.body,
        })
    }
}

#[utoipa::path(
    get,
    path = "/notes",
    operation_id = "list_notes",
    responses((status = OK, description = "All notes, most recently updated first", body = Vec<Note>))
)]
async fn list(State(pool): State<SqlitePool>) -> Result<Json<Vec<Note>>, ApiError> {
    let notes = sqlx::query_as!(
        Note,
        r#"SELECT id, title, body,
                  created_at AS "created_at: DateTime<Utc>",
                  updated_at AS "updated_at: DateTime<Utc>"
           FROM notes
           ORDER BY updated_at DESC, id DESC"#
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(notes))
}

#[utoipa::path(
    post,
    path = "/notes",
    operation_id = "create_note",
    request_body = NoteInput,
    responses(
        (status = CREATED, description = "Note created", body = Note),
        (status = UNPROCESSABLE_ENTITY, description = "Title is empty", body = ErrorBody),
    )
)]
async fn create(
    State(pool): State<SqlitePool>,
    Json(input): Json<NoteInput>,
) -> Result<(StatusCode, Json<Note>), ApiError> {
    let input = input.validate()?;
    let note = sqlx::query_as!(
        Note,
        r#"INSERT INTO notes (title, body) VALUES (?, ?)
           RETURNING id AS "id!", title, body,
                     created_at AS "created_at: DateTime<Utc>",
                     updated_at AS "updated_at: DateTime<Utc>""#,
        input.title,
        input.body
    )
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(note)))
}

#[utoipa::path(
    get,
    path = "/notes/{id}",
    operation_id = "get_note",
    params(("id" = i64, Path, description = "Note id")),
    responses(
        (status = OK, description = "The note", body = Note),
        (status = NOT_FOUND, description = "No note with this id", body = ErrorBody),
    )
)]
async fn fetch(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Note>, ApiError> {
    let note = sqlx::query_as!(
        Note,
        r#"SELECT id, title, body,
                  created_at AS "created_at: DateTime<Utc>",
                  updated_at AS "updated_at: DateTime<Utc>"
           FROM notes
           WHERE id = ?"#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(note))
}

#[utoipa::path(
    put,
    path = "/notes/{id}",
    operation_id = "update_note",
    params(("id" = i64, Path, description = "Note id")),
    request_body = NoteInput,
    responses(
        (status = OK, description = "Note updated", body = Note),
        (status = NOT_FOUND, description = "No note with this id", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Title is empty", body = ErrorBody),
    )
)]
async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<NoteInput>,
) -> Result<Json<Note>, ApiError> {
    let input = input.validate()?;
    let note = sqlx::query_as!(
        Note,
        r#"UPDATE notes
           SET title = ?, body = ?, updated_at = CURRENT_TIMESTAMP
           WHERE id = ?
           RETURNING id AS "id!", title, body,
                     created_at AS "created_at: DateTime<Utc>",
                     updated_at AS "updated_at: DateTime<Utc>""#,
        input.title,
        input.body,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(note))
}

#[utoipa::path(
    delete,
    path = "/notes/{id}",
    operation_id = "delete_note",
    params(("id" = i64, Path, description = "Note id")),
    responses(
        (status = NO_CONTENT, description = "Note deleted"),
        (status = NOT_FOUND, description = "No note with this id", body = ErrorBody),
    )
)]
async fn remove(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query!("DELETE FROM notes WHERE id = ?", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    Validation(&'static str),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    error: &'static str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "note not found"),
            Self::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            Self::Database(err) => {
                tracing::error!(error = %err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        };
        (status, Json(ErrorBody { error })).into_response()
    }
}
