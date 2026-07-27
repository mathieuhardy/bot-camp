//! Honeypot playground, plus its config, reset, and status endpoints.
//!
//! Any path under `/honeypot/` is the trap itself — a well-behaved
//! crawler should never fetch anything there, since the only way in is
//! a link hidden from real users (see `/content?hidden_link=...`).
//! `config`, `reset`, and `status` stay reachable even while banned.

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

use crate::honeypot::HoneypotConfig;
use crate::rate_limit::extract_key;
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
}

/// Axum middleware gating every request under `/honeypot/*`: a banned
/// key gets `403 Forbidden` immediately, without reaching [`trap`].
///
/// # Returns
/// [`trap`]'s response if not currently banned; `403 Forbidden` with
/// `Retry-After` otherwise.
pub(crate) async fn enforce(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let key = {
        let config = state.honeypot.config().await;
        extract_key(config.key_strategy, request.headers(), peer)
    };

    match state.honeypot.retry_after_secs(&key) {
        Some(retry_after_secs) => (
            StatusCode::FORBIDDEN,
            [(RETRY_AFTER, retry_after_secs.to_string())],
        )
            .into_response(),

        None => next.run(request).await,
    }
}

/// The trap itself: any path under `/honeypot/` lands here once past
/// [`enforce`], and springs the ban for the visiting key — the response
/// looks like an ordinary page, so nothing gives away that the key just
/// got caught.
///
/// # Returns
/// `200 OK` with a short acknowledgement.
pub async fn trap(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(path): Path<String>,
) -> String {
    let key = {
        let config = state.honeypot.config().await;
        extract_key(config.key_strategy, &headers, peer)
    };
    state.honeypot.spring(&key).await;

    format!("ok: /honeypot/{path}")
}

/// Replaces the current honeypot configuration and clears every ban.
///
/// # Returns
/// `200 OK` once the new configuration is in effect.
pub async fn set_config(
    State(state): State<AppState>,
    Json(config): Json<HoneypotConfig>,
) -> StatusCode {
    state.honeypot.configure(config).await;

    StatusCode::OK
}

/// Clears every ban, without changing the configuration — use this to
/// start a fresh test run.
///
/// # Returns
/// `200 OK` once every ban is cleared.
pub async fn reset(State(state): State<AppState>) -> StatusCode {
    state.honeypot.reset();

    StatusCode::OK
}

/// Returns introspection data for a key: its own by default, or an
/// arbitrary one via `key`.
///
/// # Returns
/// `200 OK` with the key's status as JSON.
pub async fn status(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(params): Query<StatusParams>,
) -> Json<StatusResponse> {
    let key = match params.key {
        Some(key) => key,
        None => {
            let config = state.honeypot.config().await;
            extract_key(config.key_strategy, &headers, peer)
        }
    };

    let status = state.honeypot.status(&key);

    Json(StatusResponse {
        key,
        banned: status.banned,
        retry_after_secs: status.retry_after_secs,
    })
}
