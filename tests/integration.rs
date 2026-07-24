//! Integration tests for the bot-camp server.

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

use bot_camp::app;

#[tokio::test]
async fn health_returns_200() {
    // Build request
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body.is_empty());
}

#[tokio::test]
async fn status_returns_the_requested_code() {
    // Nominal case: every valid HTTP status code, including non-standard
    // ones (100 to 999)
    for code in 100..1000 {
        // Build request
        let request = Request::builder()
            .uri(format!("/status/{code}"))
            .body(Body::empty())
            .unwrap();

        // Send request to app
        let response = app().oneshot(request).await.unwrap();

        // Verify status
        assert_eq!(response.status(), StatusCode::from_u16(code).unwrap());

        // Verify body
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());
    }
}

#[tokio::test]
async fn status_rejects_out_of_range_values() {
    for code in [-1, 0, 99, 1000] {
        // Build request
        let request = Request::builder()
            .uri(format!("/status/{code}"))
            .body(Body::empty())
            .unwrap();

        // Send request to app
        let response = app().oneshot(request).await.unwrap();

        // Verify status
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
