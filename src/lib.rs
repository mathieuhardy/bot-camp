//! Bot-camp: A self-hosted test server for crawlers and scrapers.
//!
//! This crate provides configurable HTTP responses to test crawler behavior
//! against various scenarios: HTTP codes, headers, robots.txt, redirects,
//! rate limiting, and anti-bot mechanisms.

mod challenge;
mod dashboard;
mod error;
mod honeypot;
mod logging;
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
/// rate limiting middleware, `/honeypot/{*path}` the only one gated by
/// the honeypot, and `/challenge/{*path}` the only one gated by the JS
/// challenge — each middleware's own admin endpoints stay reachable even
/// while a key is banned (or unsolved), and every other route is
/// entirely unaffected by any of them.
///
/// # Returns
/// A configured `Router` ready to serve requests.
pub fn app() -> Router {
    let state = AppState::default();

    let ratelimit_playground = Router::new()
        .route("/ratelimit/{*path}", get(routes::ratelimit_probe))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            routes::ratelimit_enforce,
        ));

    let honeypot_playground = Router::new()
        .route("/honeypot/{*path}", get(routes::honeypot_trap))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            routes::honeypot_enforce,
        ));

    let challenge_playground = Router::new()
        .route("/challenge/{*path}", get(routes::challenge_probe))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            routes::challenge_enforce,
        ));

    Router::new()
        .route("/auth/basic", get(routes::basic))
        .route("/broken-html", get(routes::broken_html))
        .route("/canonical", get(routes::canonical))
        .route("/challenge/config", put(routes::challenge_set_config))
        .route("/content", get(routes::content))
        .route("/dashboard", get(routes::dashboard_index))
        .route("/dashboard/snapshot", get(routes::dashboard_snapshot))
        .route("/dashboard/ws", get(routes::dashboard_ws))
        .route("/dashboard/{*path}", get(routes::dashboard_assets))
        .route("/delay/{ms}", get(routes::delay))
        .route("/discovery", get(routes::discovery))
        .route("/discovery/target/{n}", get(routes::discovery_target))
        .route("/encoding", get(routes::encoding))
        .route("/headers/echo", get(routes::echo))
        .route("/headers/set", get(routes::set))
        .route("/health", get(routes::health))
        .route("/honeypot/config", put(routes::honeypot_set_config))
        .route("/honeypot/reset", post(routes::honeypot_reset))
        .route("/honeypot/status", get(routes::honeypot_status))
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
        .route("/response", post(routes::response))
        .route(
            "/robots.txt",
            get(routes::robots_txt).put(routes::set_robots_txt),
        )
        .route("/robots/meta", get(routes::robots_meta))
        .route("/status/{code}", get(routes::status))
        .merge(ratelimit_playground)
        .merge(honeypot_playground)
        .merge(challenge_playground)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(logging::log_request))
}
