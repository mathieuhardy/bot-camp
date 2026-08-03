//! Rate-limited playground, plus its config, reset, and status endpoints.
//!
//! The playground (`/ratelimit/{*path}`) is the only route gated by the
//! [`enforce`] middleware — `config`, `reset`, and `status` stay
//! reachable even while a key is banned, so you can always recover.

use std::net::SocketAddr;

use axum::Json;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header::RETRY_AFTER;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use serde::Deserialize;
use serde::Serialize;

use crate::logging::with_rule;
use crate::rate_limit::Decision;
use crate::rate_limit::RateLimitConfig;
use crate::rate_limit::client_ip;
use crate::rate_limit::extract_key;
use crate::rate_limit::user_agent;
use crate::state::AppState;

/// Query parameters accepted by [`status`].
#[derive(Deserialize)]
pub(crate) struct StatusParams {
    /// The key to inspect. Defaults to the caller's own key, computed
    /// the same way as [`enforce`].
    #[serde(default)]
    key: Option<String>,
}

/// Response body returned by [`status`].
#[derive(Serialize)]
pub(crate) struct StatusResponse {
    /// The key that was inspected.
    key: String,

    /// Whether that key is currently banned.
    banned: bool,

    /// Seconds remaining on the ban, if any.
    retry_after_secs: Option<u64>,

    /// Whether the caller's IP or `User-Agent` is on the block-list.
    /// Always `false` when `key` was overridden via the query string,
    /// since block/allow-list matching works off the request's actual
    /// IP/`User-Agent`, not an arbitrary key.
    blocked: bool,

    /// Whether the caller's IP or `User-Agent` is on the allow-list.
    /// Same caveat as `blocked` for an overridden `key`.
    allow_listed: bool,
}

/// Axum middleware that gates every request behind the current rate
/// limiting configuration.
///
/// A block-list match always wins, then an allow-list match bypasses
/// the algorithm entirely (never counted, never banned); only then does
/// the configured algorithm run.
///
/// # Returns
/// The wrapped handler's response if allowed; `403 Forbidden` if the
/// IP/`User-Agent` is block-listed; `429 Too Many Requests` with
/// `Retry-After` if the algorithm's rate is exceeded; `403 Forbidden`
/// with `Retry-After` if the key is temporarily banned.
pub(crate) async fn enforce(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let config = state.rate_limit.config().await;
    let ip = client_ip(request.headers(), peer);
    let ua = user_agent(request.headers());

    if config.is_blocked(&ip, &ua) {
        return with_rule(StatusCode::FORBIDDEN.into_response(), "rate_limit_blocked");
    }

    if config.is_allow_listed(&ip, &ua) {
        return next.run(request).await;
    }

    let key = extract_key(config.key_strategy, request.headers(), peer);

    match state.rate_limit.check(&key).await {
        Decision::Allowed => next.run(request).await,

        Decision::Limited { retry_after_secs } => with_rule(
            (
                StatusCode::TOO_MANY_REQUESTS,
                [(RETRY_AFTER, retry_after_secs.to_string())],
            )
                .into_response(),
            "rate_limit_limited",
        ),

        Decision::Banned { retry_after_secs } => with_rule(
            (
                StatusCode::FORBIDDEN,
                [(RETRY_AFTER, retry_after_secs.to_string())],
            )
                .into_response(),
            "rate_limit_banned",
        ),
    }
}

/// Playground page reached once a request clears [`enforce`] — any path
/// under `/ratelimit/` lands here, so you can simulate crawling several
/// pages of a site sharing the same rate limit.
///
/// # Returns
/// `200 OK` with a short acknowledgement.
pub async fn probe(Path(path): Path<String>) -> String {
    format!("ok: /ratelimit/{path}")
}

/// Replaces the current rate limiting configuration and clears every
/// key's counters and bans, since they don't carry meaning under a new
/// policy.
///
/// # Returns
/// `200 OK` once the new configuration is in effect.
pub async fn set_config(
    State(state): State<AppState>,
    Json(config): Json<RateLimitConfig>,
) -> StatusCode {
    state.rate_limit.configure(config).await;

    StatusCode::OK
}

/// Clears every key's counters and bans, without changing the
/// configuration — use this to start a fresh test run.
///
/// # Returns
/// `200 OK` once every key's state is cleared.
pub async fn reset(State(state): State<AppState>) -> StatusCode {
    state.rate_limit.reset();

    StatusCode::OK
}

/// Returns introspection data for a key: its own by default, or an
/// arbitrary one via `key`, current quota/ban state — handy to check
/// while developing a crawler against the rate-limited playground.
///
/// # Returns
/// `200 OK` with the key's status as JSON.
pub async fn status(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(params): Query<StatusParams>,
) -> Json<StatusResponse> {
    let config = state.rate_limit.config().await;

    let (key, blocked, allow_listed) = match params.key {
        Some(key) => (key, false, false),
        None => {
            let ip = client_ip(&headers, peer);
            let ua = user_agent(&headers);
            let key = extract_key(config.key_strategy, &headers, peer);

            (
                key,
                config.is_blocked(&ip, &ua),
                config.is_allow_listed(&ip, &ua),
            )
        }
    };

    let status = state.rate_limit.status(&key);

    Json(StatusResponse {
        key,
        banned: status.banned,
        retry_after_secs: status.retry_after_secs,
        blocked,
        allow_listed,
    })
}
