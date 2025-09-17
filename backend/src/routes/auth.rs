use axum::{Json, extract::State, response::IntoResponse};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, types::uuid};
use time::OffsetDateTime;
use tracing::{error, info};

use crate::util::hash::{hash, validate};

#[derive(Deserialize)]
pub struct RegisterInput {
    username: String,
    password: String,
}

pub async fn register_handler(
    State(pool): State<PgPool>,
    Json(req): Json<RegisterInput>,
) -> Result<Json<String>, axum::http::StatusCode> {
    let password_hash: String = hash(&req.password);
    info!(password_hash);
    sqlx::query("INSERT INTO service_auth (username, password_hash) VALUES ($1, $2)")
        .bind(&req.username)
        .bind(&password_hash)
        .execute(&pool)
        .await
        .map_err(|_| axum::http::StatusCode::CONFLICT)?;
    info!("User {:?} Registered", &req.username);
    Ok(Json("User registered".to_string()))
}

#[derive(Deserialize)]
pub struct LoginInput {
    username: String,
    password: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Claims {
    sub: String,
    username: String,
    #[serde(with = "jwt_numeric_date")]
    iat: OffsetDateTime,
    #[serde(with = "jwt_numeric_date")]
    exp: OffsetDateTime,
}

impl Claims {
    /// If a token should always be equal to its representation after serializing and deserializing
    /// again, this function must be used for construction. `OffsetDateTime` contains a microsecond
    /// field but JWT timestamps are defined as UNIX timestamps (seconds). This function normalizes
    /// the timestamps.
    pub fn new(sub: String, username: String, iat: OffsetDateTime, exp: OffsetDateTime) -> Self {
        // normalize the timestamps by stripping of microseconds
        let iat = iat
            .date()
            .with_hms_milli(iat.hour(), iat.minute(), iat.second(), 0)
            .unwrap()
            .assume_utc();
        let exp = exp
            .date()
            .with_hms_milli(exp.hour(), exp.minute(), exp.second(), 0)
            .unwrap()
            .assume_utc();

        Self {
            sub,
            username,
            iat,
            exp,
        }
    }
}
mod jwt_numeric_date {
    //! Custom serialization of OffsetDateTime to conform with the JWT spec (RFC 7519 section 2, "Numeric Date")
    use serde::{self, Deserialize, Deserializer, Serializer};
    use time::OffsetDateTime;

    /// Serializes an OffsetDateTime to a Unix timestamp (milliseconds since 1970/1/1T00:00:00T)
    pub fn serialize<S>(date: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let timestamp = date.unix_timestamp();
        serializer.serialize_i64(timestamp)
    }

    /// Attempts to deserialize an i64 and use as a Unix timestamp
    pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        OffsetDateTime::from_unix_timestamp(i64::deserialize(deserializer)?)
            .map_err(|_| serde::de::Error::custom("invalid Unix timestamp value"))
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ReturnDB {
    id: uuid::Uuid,
    username: String,
    password_hash: String,
}

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

        let claims = Claims::new(sub.clone(), username, iat, exp);

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_ref()),
        )
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(token))
    } else {
        Err(axum::http::StatusCode::UNAUTHORIZED)
    }
}

#[derive(Debug, Deserialize)]
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
}

#[derive(Deserialize)]
pub struct SchoologyLogin {
    token: String,
    schoology_email: String,
    schoology_password: String,
}

pub async fn schoology_credentials_handler(
    State(pool): State<PgPool>,
    Json(req): Json<SchoologyLogin>,
) -> impl IntoResponse {
    let jwt_secret = dotenvy::var("JWT_SECRET").unwrap();
    let validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let uuid_jwt = match jsonwebtoken::decode::<Claims>(&req.token, &decoding_key, &validation)
        .map(|x| x.claims.sub)
    {
        Ok(uuid) => uuid,
        Err(err) => {
            let _ = match *err.kind() {
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
            //return (Json(msg.to_string()), axum::http::StatusCode::UNAUTHORIZED);
            return Err(axum::http::StatusCode::UNAUTHORIZED);
        }
    };
    info!("Giving Grades to: {:?}", uuid_jwt);

    let uuid = uuid::Uuid::parse_str(uuid_jwt.as_str())
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    sqlx::query!(
        "INSERT INTO schoology_auth (id, encrypted_email, encrypted_password) VALUES ($1, $2, $3)
         ON CONFLICT (id) DO UPDATE SET 
             encrypted_email = EXCLUDED.encrypted_email,
             encrypted_password = EXCLUDED.encrypted_password",
        uuid,
        crypto_utils::encrypt_string(req.schoology_email.as_str()),
        crypto_utils::encrypt_string(req.schoology_password.as_str()),
    )
    .execute(&pool)
    .await
    .map_err(|err| {
        error!(
            "Failed to store Schoology credentials for user {}: {}",
            uuid, err
        );
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}
