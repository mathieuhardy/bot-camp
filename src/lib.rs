//! Bot-camp: A self-hosted test server for crawlers and scrapers.
//!
//! This crate provides configurable HTTP responses to test crawler behavior
//! against various scenarios: HTTP codes, headers, robots.txt, redirects,
//! rate limiting, and anti-bot mechanisms.

mod error;
mod rate_limit;
mod routes;
mod state;
mod templates;

use axum::Router;
use axum::middleware;
use axum::routing::get;
use axum::routing::post;
use axum::routing::put;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use crate::error::{Error, Result};
use crate::state::AppState;

/// Creates the application router with all routes and middleware.
///
/// The `/ratelimit/{*path}` playground is the only route gated by the
/// rate limiting middleware — `/ratelimit/config`, `/ratelimit/reset`,
/// and `/ratelimit/status` stay reachable even while a key is banned,
/// and every other route is entirely unaffected by rate limiting.
///
/// # Returns
/// A configured `Router` ready to serve requests.
pub fn app() -> Router {
    let state = AppState::default();

    let playground = Router::new()
        .route("/ratelimit/{*path}", get(routes::ratelimit_probe))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            routes::ratelimit_enforce,
        ));

    Router::new()
        .route("/auth/basic", get(routes::basic))
        .route("/broken-html", get(routes::broken_html))
        .route("/canonical", get(routes::canonical))
        .route("/content", get(routes::content))
        .route("/delay/{ms}", get(routes::delay))
        .route("/encoding", get(routes::encoding))
        .route("/headers/echo", get(routes::echo))
        .route("/headers/set", get(routes::set))
        .route("/health", get(routes::health))
        .route("/js-render", get(routes::js_render))
        .route("/large-response/{kb}", get(routes::large_response))
        .route("/normalize", get(routes::normalize))
        .route("/ratelimit/config", put(routes::ratelimit_set_config))
        .route("/ratelimit/reset", post(routes::ratelimit_reset))
        .route("/ratelimit/status", get(routes::ratelimit_status))
        .route("/redirect/chain", get(routes::redirect_chain))
        .route("/redirect/loop", get(routes::redirect_loop))
        .route("/redirect/meta-refresh", get(routes::redirect_meta_refresh))
        .route("/redirect/refresh", get(routes::redirect_refresh))
        .route("/redirect/{code}", get(routes::redirect))
        .route(
            "/robots.txt",
            get(routes::robots_txt).put(routes::set_robots_txt),
        )
        .route("/robots/meta", get(routes::robots_meta))
        .route("/status/{code}", get(routes::status))
        .merge(playground)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
}
