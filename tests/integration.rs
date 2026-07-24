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

#[tokio::test]
async fn headers_echo_returns_received_headers_as_json() {
    // Build request
    let request = Request::builder()
        .uri("/headers/echo")
        .header("x-foo", "bar")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#""x-foo":["bar"]"#));
}

#[tokio::test]
async fn headers_set_appends_one_header_line_per_query_param() {
    // Build request
    let request = Request::builder()
        .uri("/headers/set?x-foo=bar&x-foo=baz")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify headers
    let values: Vec<_> = response.headers().get_all("x-foo").iter().collect();
    assert_eq!(values, vec!["bar", "baz"]);
}

#[tokio::test]
async fn headers_set_rejects_an_invalid_header_name() {
    // Build request: "bad header" (percent-encoded space isn't a valid
    // header name)
    let request = Request::builder()
        .uri("/headers/set?bad%20header=value")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn headers_set_rejects_an_invalid_header_value() {
    // Build request: an embedded newline isn't a valid header value
    let request = Request::builder()
        .uri("/headers/set?x-foo=bar%0Abaz")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
