//! Controlled response-size endpoint.

use axum::extract::Path;

/// Number of bytes in one kilobyte, as used to size the response body.
const BYTES_PER_KB: u64 = 1024;

/// Returns a response body of exactly `kb` kilobytes, filled with `'a'`
/// characters, to test how a crawler handles very large or very small
/// pages.
///
/// # Returns
/// `200 OK` with a body of `kb * 1024` bytes.
pub async fn large_response(Path(kb): Path<u64>) -> String {
    "a".repeat((kb * BYTES_PER_KB) as usize)
}

#[cfg(test)]
mod tests {
    use axum::extract::Path;

    use super::large_response;

    #[tokio::test]
    async fn returns_a_body_of_the_requested_size() {
        let body = large_response(Path(2)).await;

        assert_eq!(body.len(), 2048);
    }

    #[tokio::test]
    async fn returns_an_empty_body_for_zero_kb() {
        let body = large_response(Path(0)).await;

        assert!(body.is_empty());
    }
}
