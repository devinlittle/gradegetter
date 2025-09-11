use std::{sync::Arc, time::Duration};

use axum::{Router, http::HeaderValue};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tracing::{Level, info};

mod routes;
mod util;

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        //        .allow_origin(HeaderValue::from_static("http://localhost:5173"))
        .allow_origin(Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers(Any);

    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();

    let database_string = dotenvy::var("DATABASE_URL").expect("DATABASE_URL not found");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_string)
        .await
        .expect("can't connect to database");

    let app = Router::new().merge(routes::create_routes(pool.clone()).layer(cors));

    let host_on = format!("0.0.0.0:{}", dotenvy::var("PORT").unwrap());

    let listener = tokio::net::TcpListener::bind(host_on).await.unwrap();

    info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
