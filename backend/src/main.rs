use std::time::Duration;

use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use hyper::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
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
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls cryptoi provider");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

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

    let production_enviorment = dotenvy::var("PRODUCTION").is_ok();

    let app = Router::new().merge(routes::create_routes(pool.clone()).layer(cors));

    let host_on = format!("0.0.0.0:{}", dotenvy::var("PORT").unwrap());

    let handle = axum_server::Handle::new();
    let shutdown_signal_handler = shutdown_signal(handle.clone());

    if production_enviorment {
        let config = RustlsConfig::from_pem_file("/etc/fullchain.pem", "/etc/privkey.pem")
            .await
            .map_err(|e| tracing::error!("failed to load RustlsConfig: {}", e))
            .unwrap();

        tokio::spawn(shutdown_signal_handler);

        let listener_std = std::net::TcpListener::bind(host_on).unwrap();
        info!("Listening on {}", listener_std.local_addr().unwrap());
        axum_server::from_tcp_rustls(listener_std, config)
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        let listener_tokio = tokio::net::TcpListener::bind(host_on).await.unwrap();

        info!("Listening on {}", listener_tokio.local_addr().unwrap());
        axum::serve(listener_tokio, app)
            .with_graceful_shutdown(shutdown_signal_handler)
            .await
            .unwrap();
    }
}

async fn shutdown_signal(handle: axum_server::Handle) {
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
