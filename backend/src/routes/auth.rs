use anyhow::Result;
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, types::uuid};
use time::OffsetDateTime;
use tracing::{error, info};
use utoipa::ToSchema;

use crate::{
    middleware::jwt::{AuthenticatedUser, Claims},
    util::hash::{hash, validate},
};

#[derive(Deserialize, ToSchema)]
pub struct RegisterInput {
    #[schema(example = "user")]
    username: String,
    #[schema(example = "password")]
    password: String,
}

#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterInput,
    responses(
        (status = 200, description = "Registers User!", body = String),
        (status = 409, description = "User exists")
    ),
    tag = "user_auth"
)]
pub async fn register_handler(
    State(pool): State<PgPool>,
    Json(req): Json<RegisterInput>,
) -> Result<Json<String>, axum::http::StatusCode> {
    let password_hash: String = hash(&req.password);
    sqlx::query("INSERT INTO service_auth (username, password_hash) VALUES ($1, $2)")
        .bind(&req.username)
        .bind(&password_hash)
        .execute(&pool)
        .await
        .map_err(|_| axum::http::StatusCode::CONFLICT)?;
    info!("User {:?} Registered", &req.username);
    Ok(Json("User registered".to_string()))
}

#[derive(Deserialize, ToSchema)]
pub struct LoginInput {
    #[schema(example = "user")]
    username: String,
    #[schema(example = "password")]
    password: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ReturnDB {
    id: uuid::Uuid,
    username: String,
    password_hash: String,
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginInput, 
    responses(
        (status = 200, description = "Returns Valid JWT for User", body = String),
        (status = 401, description = "Credentials Incorrect"),
        (status = 500, description = "Interal Server Error")
    ),
    tag = "user_auth"
)]
pub async fn login_handler(
    State(pool): State<PgPool>,
    Json(req): Json<LoginInput>,
) -> impl IntoResponse {
    let row = sqlx::query_as!(
        ReturnDB,
        "SELECT id, password_hash, username FROM service_auth WHERE username = $1",
        &req.username
    )
    .fetch_optional(&pool)
    .await
    .map_err(|err| {
        tracing::info!("Database error: {}", err);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let user = match row {
        Some(user) => user,
        None => return Err(axum::http::StatusCode::UNAUTHORIZED), // User not found
    };

    let jwt_secret = dotenvy::var("JWT_SECRET").unwrap();

    if validate(req.password.as_str(), &user.password_hash) {
        let sub = user.id.to_string();
        let username = req.username.to_string();
        let iat = OffsetDateTime::now_utc();
        let exp = iat + time::Duration::days(365);

        let claims = Claims {
            sub: sub.clone(),
            username,
            iat,
            exp,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_ref()),
        )
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

        info!(r#"User "{}" logged in sucessfully"#, user.username);
        Ok(Json(token))
    } else {
        info!(
            r#"User failed to login; username: "{0}", id: "{1}""#,
            user.username, user.id
        );
        Err(axum::http::StatusCode::UNAUTHORIZED)
    }
}

#[derive(Deserialize, ToSchema)]
pub struct SchoologyLogin {
    #[schema(example = "email@exmaple.com")]
    schoology_email: String,
    #[schema(example = "password")]
    schoology_password: String,
}

#[utoipa::path(
    post,
    path = "/auth/schoology/credentials",
    request_body = SchoologyLogin,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Encrypts schoology info and inserts into database", body = String),
        (status = 401, description = "Credentials Incorrect"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "user_auth"
)]
pub async fn schoology_credentials_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<SchoologyLogin>,
) -> Result<(), StatusCode> {
    info!(
        "Encrypted Schoology Credentials added to user: {:?}",
        user.username
    );

    sqlx::query!(
        "INSERT INTO schoology_auth (id, encrypted_email, encrypted_password) VALUES ($1, $2, $3)
         ON CONFLICT (id) DO UPDATE SET 
             encrypted_email = EXCLUDED.encrypted_email,
             encrypted_password = EXCLUDED.encrypted_password",
        user.uuid,
        crypto_utils::encrypt_string(req.schoology_email.as_str()),
        crypto_utils::encrypt_string(req.schoology_password.as_str()),
    )
    .execute(&pool)
    .await
    .map_err(|err| {
        error!(
            "Failed to store Schoology credentials for user {}: {}",
            user.uuid, err
        );
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/auth/forward",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Initilized User on GradeGetter", body = String),
        (status = 401, description = "Credentials Incorrect"),
        (status = 500, description = "Interal Server Error")
    ),
    tag = "user_auth"
)]
pub async fn foward_to_gradegetter() -> Result<(), StatusCode> {
    let client = reqwest::Client::new();
    let _ = client
        .request(reqwest::Method::GET, "http://gradegetter:3001/userinit")
        .send()
        .await
        .map_err(|err| {
            error!("failed to initlize user... {}", err);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(())
}


#[utoipa::path(
    delete,
    path = "/auth/delete",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Deleted User", body = String),
        (status = 401, description = "Credentials Incorrect"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Interal Server Error")
    ),
    tag = "user_auth"
)]
pub async fn delete_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    match sqlx::query!("DELETE FROM service_auth WHERE id = $1", user.uuid)
        .execute(&pool)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => {
            info!("deleted user: {}", user.username);
            axum::http::StatusCode::OK
        }
        Ok(_) => StatusCode::NOT_FOUND,
        Err(err) => {
            error!("database error: {:?}", err);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/*#[derive(Debug, Deserialize)]
pub struct ValidateInput {
    token: String,
}

pub async fn validate_token(Json(req): Json<ValidateInput>) -> impl IntoResponse {
    let validation = Validation::new(jsonwebtoken::Algorithm::HS256);

    let jwt_secret = dotenvy::var("JWT_SECRET").unwrap();
    let _token_data = match decode::<Claims>(
        req.token.as_str(),
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    ) {
        Ok(c) => c,
        Err(err) => {
            let msg = match *err.kind() {
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    tracing::warn!("InvalidToken");
                    "Invalid Token"
                }
                jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    tracing::warn!("InvalidSignature");
                    "Invalid Signature"
                }
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    tracing::warn!("ExpiredSignature");
                    "Expiered Signature"
                }
                _ => {
                    tracing::warn!("Something really bad happened");
                    "Token Verifation fail"
                }
            };
            return (axum::http::StatusCode::UNAUTHORIZED, Json(msg.to_string()));
        }
    };
    (axum::http::StatusCode::OK, Json("Good Token!".to_string()))
} */
