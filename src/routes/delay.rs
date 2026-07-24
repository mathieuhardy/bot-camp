//! Response-delay endpoint.

use std::time::Duration;

use axum::extract::Path;
use axum::http::StatusCode;
use tokio::time::sleep;

/// Waits `ms` milliseconds before responding, to simulate a slow page
/// load.
///
/// # Returns
/// `200 OK` with an empty body, after the requested delay.
pub async fn delay(Path(ms): Path<u64>) -> StatusCode {
    sleep(Duration::from_millis(ms)).await;

    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use axum::extract::Path;
    use axum::http::StatusCode;

    use super::delay;

    #[tokio::test]
    async fn waits_for_the_requested_duration() {
        let start = Instant::now();

        let result = delay(Path(20)).await;

        assert_eq!(result, StatusCode::OK);
        assert!(start.elapsed().as_millis() >= 20);
    }

    #[tokio::test]
    async fn returns_immediately_for_a_zero_delay() {
        let result = delay(Path(0)).await;

        assert_eq!(result, StatusCode::OK);
    }
}
