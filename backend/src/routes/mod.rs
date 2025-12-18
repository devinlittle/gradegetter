use axum::{
    Router,
    routing::{delete, get, post},
};
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod auth;
pub mod grades;

#[derive(OpenApi)]
#[openapi(
      paths(
        // Auth paths
        crate::routes::auth::register_handler,
        crate::routes::auth::login_handler,
        crate::routes::auth::delete_handler,
        crate::routes::auth::foward_to_gradegetter,
        crate::routes::auth::schoology_credentials_handler,
        // Grade path
        crate::routes::grades::grades_handler,
    ),
    components(
        schemas(
            crate::routes::auth::RegisterInput,
            crate::routes::auth::LoginInput,
            crate::middleware::jwt::AuthenticatedUser,
            crate::middleware::jwt::Claims,
        )
    ),
    modifiers(&JwtBearer),
    tags(
        (name = "user_auth", description = "Authentication endpoints"),
        (name = "grades", description = "Grade Endpoints")
    )
)]
pub struct DaApiDoc;

struct JwtBearer;

impl utoipa::Modify for JwtBearer {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            )
        }
    }
}

pub fn create_routes(pool: PgPool) -> Router {
    let routes_without_middleware = Router::new()
        .route("/auth/register", post(auth::register_handler))
        .route("/auth/login", post(auth::login_handler));

    let routes_with_middleware = Router::new()
        // Auth Routes
        .route("/auth/delete", delete(auth::delete_handler))
        .route("/auth/forward", get(auth::foward_to_gradegetter))
        .route(
            "/auth/schoology/credentials",
            post(auth::schoology_credentials_handler),
        )
        // Grade Route
        .route("/grades", get(grades::grades_handler))
        .layer(axum::middleware::from_fn(crate::middleware::jwt::jwt_auth));

    Router::new()
        .merge(routes_with_middleware)
        .merge(routes_without_middleware)
        .merge(SwaggerUi::new("/swegger-ui").url("/api-docs/openapi.json", DaApiDoc::openapi()))
        .with_state(pool)
}
