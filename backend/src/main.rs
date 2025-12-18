use axum::Router;
use hyper::header::{AUTHORIZATION, CONTENT_TYPE};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, time::Duration};
use tokio::signal::{
    self,
    unix::{SignalKind, signal},
};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod middleware;
mod routes;
mod util;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls cryptoi provider");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let database_string = dotenvy::var("DATABASE_URL").expect("DATABASE_URL env_var not found");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_string)
        .await
        .expect("can't connect to database");

    let app = Router::new().merge(routes::create_routes(pool.clone()).layer(cors));

    let host_on = format!(
        "0.0.0.0:{}",
        dotenvy::var("PORT").expect("PORT env var not found")
    );

    let handle = axum_server::Handle::new();
    let shutdown_signal_handler = shutdown_signal(handle.clone());

    let listener_tokio = tokio::net::TcpListener::bind(host_on).await.unwrap();

    info!("Listening on {}", listener_tokio.local_addr().unwrap());
    axum::serve(listener_tokio, app)
        .with_graceful_shutdown(shutdown_signal_handler)
        .await
        .unwrap();
}

async fn shutdown_signal(handle: axum_server::Handle<SocketAddr>) {
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
    handle.graceful_shutdown(Some(Duration::from_secs(10)));
}
