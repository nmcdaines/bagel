use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::{ApiError, ApiJson, ApiPath, ApiQuery, ErrorBody};

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
    /// The project this note belongs to, if any.
    pub project_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NoteInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// The project to file the note under; omit or `null` for none.
    #[serde(default)]
    pub project_id: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NoteFilter {
    /// Only return notes belonging to this project.
    pub project_id: Option<i64>,
}

impl NoteInput {
    fn validate(self) -> Result<Self, ApiError> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err(ApiError::Validation("title must not be empty"));
        }
        Ok(Self {
            title: title.to_owned(),
            ..self
        })
    }
}

/// Reports a foreign key violation on `notes.project_id` as a 422.
fn project_must_exist(err: sqlx::Error) -> ApiError {
    match err.as_database_error() {
        Some(db) if db.is_foreign_key_violation() => ApiError::Validation("project does not exist"),
        _ => err.into(),
    }
}

#[utoipa::path(
    get,
    path = "/notes",
    operation_id = "list_notes",
    params(NoteFilter),
    responses(
        (status = OK, description = "Notes, most recently updated first", body = Vec<Note>),
        (status = BAD_REQUEST, description = "Query parameters are invalid", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn list(
    State(pool): State<SqlitePool>,
    ApiQuery(filter): ApiQuery<NoteFilter>,
) -> Result<Json<Vec<Note>>, ApiError> {
    let notes = sqlx::query_as!(
        Note,
        r#"SELECT id, title, body, project_id AS "project_id?",
                  created_at AS "created_at: DateTime<Utc>",
                  updated_at AS "updated_at: DateTime<Utc>"
           FROM notes
           WHERE ?1 IS NULL OR project_id = ?1
           ORDER BY updated_at DESC, id DESC"#,
        filter.project_id
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
        (status = BAD_REQUEST, description = "Request body is not valid JSON", body = ErrorBody),
        (status = PAYLOAD_TOO_LARGE, description = "Request body is too large", body = ErrorBody),
        (status = UNSUPPORTED_MEDIA_TYPE, description = "Content-Type is not application/json", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Body does not match NoteInput, title is empty, or project does not exist", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn create(
    State(pool): State<SqlitePool>,
    ApiJson(input): ApiJson<NoteInput>,
) -> Result<(StatusCode, Json<Note>), ApiError> {
    let input = input.validate()?;
    let note = sqlx::query_as!(
        Note,
        r#"INSERT INTO notes (title, body, project_id) VALUES (?, ?, ?)
           RETURNING id AS "id!", title, body, project_id AS "project_id?",
                     created_at AS "created_at: DateTime<Utc>",
                     updated_at AS "updated_at: DateTime<Utc>""#,
        input.title,
        input.body,
        input.project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(project_must_exist)?;
    Ok((StatusCode::CREATED, Json(note)))
}

#[utoipa::path(
    get,
    path = "/notes/{id}",
    operation_id = "get_note",
    params(("id" = i64, Path, description = "Note id")),
    responses(
        (status = OK, description = "The note", body = Note),
        (status = BAD_REQUEST, description = "Id is not an integer", body = ErrorBody),
        (status = NOT_FOUND, description = "No note with this id", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn fetch(
    State(pool): State<SqlitePool>,
    ApiPath(id): ApiPath<i64>,
) -> Result<Json<Note>, ApiError> {
    let note = sqlx::query_as!(
        Note,
        r#"SELECT id, title, body, project_id AS "project_id?",
                  created_at AS "created_at: DateTime<Utc>",
                  updated_at AS "updated_at: DateTime<Utc>"
           FROM notes
           WHERE id = ?"#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(ApiError::NotFound("note"))?;
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
        (status = BAD_REQUEST, description = "Id is not an integer, or request body is not valid JSON", body = ErrorBody),
        (status = NOT_FOUND, description = "No note with this id", body = ErrorBody),
        (status = PAYLOAD_TOO_LARGE, description = "Request body is too large", body = ErrorBody),
        (status = UNSUPPORTED_MEDIA_TYPE, description = "Content-Type is not application/json", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Body does not match NoteInput, title is empty, or project does not exist", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn update(
    State(pool): State<SqlitePool>,
    ApiPath(id): ApiPath<i64>,
    ApiJson(input): ApiJson<NoteInput>,
) -> Result<Json<Note>, ApiError> {
    let input = input.validate()?;
    let note = sqlx::query_as!(
        Note,
        r#"UPDATE notes
           SET title = ?, body = ?, project_id = ?, updated_at = CURRENT_TIMESTAMP
           WHERE id = ?
           RETURNING id AS "id!", title, body, project_id AS "project_id?",
                     created_at AS "created_at: DateTime<Utc>",
                     updated_at AS "updated_at: DateTime<Utc>""#,
        input.title,
        input.body,
        input.project_id,
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(project_must_exist)?
    .ok_or(ApiError::NotFound("note"))?;
    Ok(Json(note))
}

#[utoipa::path(
    delete,
    path = "/notes/{id}",
    operation_id = "delete_note",
    params(("id" = i64, Path, description = "Note id")),
    responses(
        (status = NO_CONTENT, description = "Note deleted"),
        (status = BAD_REQUEST, description = "Id is not an integer", body = ErrorBody),
        (status = NOT_FOUND, description = "No note with this id", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn remove(
    State(pool): State<SqlitePool>,
    ApiPath(id): ApiPath<i64>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query!("DELETE FROM notes WHERE id = ?", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("note"));
    }
    Ok(StatusCode::NO_CONTENT)
}
