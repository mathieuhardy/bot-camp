//! Generic HTTP status code endpoint.

use axum::extract::Path;
use axum::http::StatusCode;

use crate::error::Result;

/// Returns the given HTTP status code.
///
/// # Returns
/// The requested `StatusCode`, or an `Error` if `code` isn't a valid HTTP
/// status code (100-999).
pub async fn status(Path(code): Path<u16>) -> Result<StatusCode> {
    StatusCode::from_u16(code).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use axum::extract::Path;
    use axum::http::StatusCode;

    use super::status;

    #[tokio::test]
    async fn returns_the_requested_status_code() {
        let result = status(Path(404)).await.unwrap();

        assert_eq!(result, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn rejects_a_code_below_100() {
        let result = status(Path(99)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn accepts_a_non_standard_code_up_to_999() {
        let result = status(Path(999)).await.unwrap();

        assert_eq!(result.as_u16(), 999);
    }
}
