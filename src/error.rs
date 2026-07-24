//! Error types for the bot-camp server.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use thiserror::Error;

/// A specialized `Result` type for bot-camp operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the bot-camp server.
#[derive(Debug, Error)]
pub enum Error {
    /// The requested value isn't a valid HTTP status code.
    #[error(transparent)]
    InvalidStatusCode(#[from] axum::http::status::InvalidStatusCode),

    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match &self {
            Error::InvalidStatusCode(_) => StatusCode::BAD_REQUEST,
            Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::Error;

    #[test]
    fn invalid_status_code_maps_to_bad_request() {
        let error = Error::from(StatusCode::from_u16(0).unwrap_err());

        assert_eq!(error.into_response().status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn io_error_maps_to_internal_server_error() {
        let error = Error::from(std::io::Error::other("boom"));

        assert_eq!(
            error.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
