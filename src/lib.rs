//! Bot-camp: A self-hosted test server for crawlers and scrapers.
//!
//! This crate provides configurable HTTP responses to test crawler behavior
//! against various scenarios: HTTP codes, headers, robots.txt, redirects,
//! rate limiting, and anti-bot mechanisms.

mod error;
mod routes;
mod templates;

use axum::Router;
use axum::routing::get;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use crate::error::{Error, Result};

/// Creates the application router with all routes and middleware.
///
/// # Returns
/// A configured `Router` ready to serve requests.
pub fn app() -> Router {
    Router::new()
        .route("/auth/basic", get(routes::basic))
        .route("/canonical", get(routes::canonical))
        .route("/delay/{ms}", get(routes::delay))
        .route("/headers/echo", get(routes::echo))
        .route("/headers/set", get(routes::set))
        .route("/health", get(routes::health))
        .route("/large-response/{kb}", get(routes::large_response))
        .route("/normalize", get(routes::normalize))
        .route("/redirect/chain", get(routes::redirect_chain))
        .route("/redirect/loop", get(routes::redirect_loop))
        .route("/redirect/meta-refresh", get(routes::redirect_meta_refresh))
        .route("/redirect/refresh", get(routes::redirect_refresh))
        .route("/redirect/{code}", get(routes::redirect))
        .route("/status/{code}", get(routes::status))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
}
