//! Health check endpoint.

use axum::http::StatusCode;

/// Health check handler.
///
/// Returns a 200 OK status to indicate the server is running.
///
/// # Returns
/// A StatusCode indicating server health.
pub async fn health() -> StatusCode {
    StatusCode::OK
}
