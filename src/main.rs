//! Bot-camp: A self-hosted test server for crawlers and scrapers.
//!
//! This server provides configurable HTTP responses to test crawler behavior
//! against various scenarios: HTTP codes, headers, robots.txt, redirects,
//! rate limiting, and anti-bot mechanisms.

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use bot_camp::app;

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
    axum::serve(
        listener,
        app().into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
