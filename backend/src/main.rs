use std::time::Duration;

use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tokio::signal::{
    self,
    unix::{SignalKind, signal},
};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

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
        .with_env_filter(EnvFilter::from_default_env())
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
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = signal::ctrl_c();

    let terminte = async {
        signal(SignalKind::terminate())
            .expect("failed to install the SIGTERM handler 🥲")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminte => {},

    }

    info!("Signal recvived now starting graceful shutdown");
}
