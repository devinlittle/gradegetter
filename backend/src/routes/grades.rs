use axum::{Json, extract::State};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;
use tracing::info;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Claims {
    sub: String,
    username: String,
    #[serde(with = "jwt_numeric_date")]
    iat: OffsetDateTime,
    #[serde(with = "jwt_numeric_date")]
    exp: OffsetDateTime,
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

pub async fn grades_handler(
    State(pool): State<PgPool>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, axum::http::StatusCode> {
    let jwt_secret = dotenvy::var("JWT_SECRET").unwrap();
    let validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let uuid = match jsonwebtoken::decode::<Claims>(bearer.token(), &decoding_key, &validation)
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

    let uuid =
        uuid::Uuid::parse_str(uuid.as_str()).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let grades_row = sqlx::query!("SELECT grades FROM grades WHERE id = $1", uuid)
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

    info!("Giving Grades to: {:?}", uuid);
    Ok(Json(grades))
}
