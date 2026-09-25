use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::{ApiError, ApiJson, ApiPath, ErrorBody};

pub fn router() -> OpenApiRouter<SqlitePool> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(fetch, update, remove))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProjectInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

impl ProjectInput {
    fn validate(self) -> Result<Self, ApiError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(ApiError::Validation("name must not be empty"));
        }
        Ok(Self {
            name: name.to_owned(),
            ..self
        })
    }
}

#[utoipa::path(
    get,
    path = "/projects",
    operation_id = "list_projects",
    responses(
        (status = OK, description = "All projects, most recently updated first", body = Vec<Project>),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn list(State(pool): State<SqlitePool>) -> Result<Json<Vec<Project>>, ApiError> {
    let projects = sqlx::query_as!(
        Project,
        r#"SELECT id, name, description,
                  created_at AS "created_at: DateTime<Utc>",
                  updated_at AS "updated_at: DateTime<Utc>"
           FROM projects
           ORDER BY updated_at DESC, id DESC"#
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(projects))
}

#[utoipa::path(
    post,
    path = "/projects",
    operation_id = "create_project",
    request_body = ProjectInput,
    responses(
        (status = CREATED, description = "Project created", body = Project),
        (status = BAD_REQUEST, description = "Request body is not valid JSON", body = ErrorBody),
        (status = PAYLOAD_TOO_LARGE, description = "Request body is too large", body = ErrorBody),
        (status = UNSUPPORTED_MEDIA_TYPE, description = "Content-Type is not application/json", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Body does not match ProjectInput, or name is empty", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn create(
    State(pool): State<SqlitePool>,
    ApiJson(input): ApiJson<ProjectInput>,
) -> Result<(StatusCode, Json<Project>), ApiError> {
    let input = input.validate()?;
    let project = sqlx::query_as!(
        Project,
        r#"INSERT INTO projects (name, description) VALUES (?, ?)
           RETURNING id AS "id!", name, description,
                     created_at AS "created_at: DateTime<Utc>",
                     updated_at AS "updated_at: DateTime<Utc>""#,
        input.name,
        input.description
    )
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(project)))
}

#[utoipa::path(
    get,
    path = "/projects/{id}",
    operation_id = "get_project",
    params(("id" = i64, Path, description = "Project id")),
    responses(
        (status = OK, description = "The project", body = Project),
        (status = BAD_REQUEST, description = "Id is not an integer", body = ErrorBody),
        (status = NOT_FOUND, description = "No project with this id", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn fetch(
    State(pool): State<SqlitePool>,
    ApiPath(id): ApiPath<i64>,
) -> Result<Json<Project>, ApiError> {
    let project = sqlx::query_as!(
        Project,
        r#"SELECT id, name, description,
                  created_at AS "created_at: DateTime<Utc>",
                  updated_at AS "updated_at: DateTime<Utc>"
           FROM projects
           WHERE id = ?"#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(ApiError::NotFound("project"))?;
    Ok(Json(project))
}

#[utoipa::path(
    put,
    path = "/projects/{id}",
    operation_id = "update_project",
    params(("id" = i64, Path, description = "Project id")),
    request_body = ProjectInput,
    responses(
        (status = OK, description = "Project updated", body = Project),
        (status = BAD_REQUEST, description = "Id is not an integer, or request body is not valid JSON", body = ErrorBody),
        (status = NOT_FOUND, description = "No project with this id", body = ErrorBody),
        (status = PAYLOAD_TOO_LARGE, description = "Request body is too large", body = ErrorBody),
        (status = UNSUPPORTED_MEDIA_TYPE, description = "Content-Type is not application/json", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Body does not match ProjectInput, or name is empty", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn update(
    State(pool): State<SqlitePool>,
    ApiPath(id): ApiPath<i64>,
    ApiJson(input): ApiJson<ProjectInput>,
) -> Result<Json<Project>, ApiError> {
    let input = input.validate()?;
    let project = sqlx::query_as!(
        Project,
        r#"UPDATE projects
           SET name = ?, description = ?, updated_at = CURRENT_TIMESTAMP
           WHERE id = ?
           RETURNING id AS "id!", name, description,
                     created_at AS "created_at: DateTime<Utc>",
                     updated_at AS "updated_at: DateTime<Utc>""#,
        input.name,
        input.description,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(ApiError::NotFound("project"))?;
    Ok(Json(project))
}

/// Deleting a project keeps its notes; they become unassigned.
#[utoipa::path(
    delete,
    path = "/projects/{id}",
    operation_id = "delete_project",
    params(("id" = i64, Path, description = "Project id")),
    responses(
        (status = NO_CONTENT, description = "Project deleted; its notes are kept, unassigned"),
        (status = BAD_REQUEST, description = "Id is not an integer", body = ErrorBody),
        (status = NOT_FOUND, description = "No project with this id", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, description = "Database error", body = ErrorBody),
    )
)]
async fn remove(
    State(pool): State<SqlitePool>,
    ApiPath(id): ApiPath<i64>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query!("DELETE FROM projects WHERE id = ?", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("project"));
    }
    Ok(StatusCode::NO_CONTENT)
}
