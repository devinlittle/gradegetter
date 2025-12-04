use axum::{Extension, Json, extract::State};
use serde_json::Value;
use sqlx::PgPool;
use tracing::info;

use crate::middleware::jwt::AuthenticatedUser;

pub async fn grades_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Value>, axum::http::StatusCode> {
    let grades_row = sqlx::query!("SELECT grades FROM grades WHERE id = $1", user.uuid)
        .fetch_optional(&pool)
        .await
        .map_err(|err| {
            tracing::info!("Database error: {}", err);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let grades = match grades_row.map(|x| x.grades.unwrap()) {
        Some(grades) => grades,
        None => return Err(axum::http::StatusCode::BAD_REQUEST),
    };

    info!("Giving Grades to: {:?}", user.username);
    Ok(Json(grades))
}
