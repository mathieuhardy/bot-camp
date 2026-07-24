//! Bot-camp: A self-hosted test server for crawlers and scrapers.
//!
//! This server provides configurable HTTP responses to test crawler behavior
//! against various scenarios: HTTP codes, headers, robots.txt, redirects,
//! rate limiting, and anti-bot mechanisms.

mod error;
mod routes;

use std::net::SocketAddr;

use axum::Router;
use axum::routing::get;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Creates the application router with all routes and middleware.
///
/// # Returns
/// A configured `Router` ready to serve requests.
pub fn app() -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/status/{code}", get(routes::status))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Bind address
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Starting server on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app()).await.unwrap();
}
