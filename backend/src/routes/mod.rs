use axum::{Router, routing::post};
use sqlx::PgPool;

pub mod auth;
pub mod grades;

pub fn create_routes(pool: PgPool) -> Router {
    Router::new()
        // Auth Routes
        .route("/auth/register", post(auth::register_handler))
        .route("/auth/login", post(auth::login_handler))
        .route("/auth/validate", post(auth::validate_token))
        .route(
            "/auth/schoology/credentials",
            post(auth::schoology_credentials_handler),
        )
        // Grade Route
        .route("/grades", post(grades::grades_handler))
        .with_state(pool)
}
