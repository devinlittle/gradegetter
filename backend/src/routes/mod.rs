use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;

pub mod auth;
pub mod grades;

pub fn create_routes(pool: PgPool) -> Router {
    Router::new()
        // Auth Routes
        .route("/auth/register", post(auth::register_handler))
        .route("/auth/login", post(auth::login_handler))
        .route("/auth/validate", post(auth::validate_token))
        .route("/auth/forward", post(auth::foward_to_gradegetter))
        .route(
            "/auth/schoology/credentials",
            post(auth::schoology_credentials_handler),
        )
        // Grade Route
        .route("/grades", get(grades::grades_handler))
        .with_state(pool)
}
