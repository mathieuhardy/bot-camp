//! Structured, per-request access logging.

use std::net::SocketAddr;
use std::time::Instant;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::rate_limit::client_ip;
use crate::rate_limit::user_agent;

/// The rule a middleware applied to reach a response — a rate limit
/// decision, a honeypot trap, a challenge block, etc. Any middleware can
/// attach this to its response via [`with_rule`]; [`log_request`] then
/// reports it without needing to know about any specific middleware.
#[derive(Clone)]
pub(crate) struct AppliedRule(pub(crate) &'static str);

/// Attaches `rule` to `response`, so [`log_request`] reports it.
pub(crate) fn with_rule(mut response: Response, rule: &'static str) -> Response {
    response.extensions_mut().insert(AppliedRule(rule));

    response
}

/// Axum middleware wrapping the whole router: logs one structured line
/// per request (method, path, IP, `User-Agent`, status, latency, and the
/// rule applied if any) — the source of truth to replay/analyze how a
/// crawler under test actually behaved.
pub(crate) async fn log_request(request: Request<Body>, next: Next) -> Response {
    // Resolve the request's identity before it's consumed by `next`. Not
    // every request carries a `ConnectInfo` (e.g. tests built without
    // one), hence the fallback.
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(|| SocketAddr::from(([0, 0, 0, 0], 0)), |info| info.0);
    let ip = client_ip(request.headers(), peer);
    let user_agent = user_agent(request.headers());
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Run the request through the rest of the stack
    let start = Instant::now();
    let response = next.run(request).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    let rule = response
        .extensions()
        .get::<AppliedRule>()
        .map_or("none", |applied| applied.0);

    tracing::info!(
        %method,
        %path,
        %ip,
        %user_agent,
        status = response.status().as_u16(),
        latency_ms,
        rule,
        "request",
    );

    response
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use axum::middleware;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use tower::ServiceExt;

    use super::with_rule;

    fn router() -> Router {
        Router::new()
            .route(
                "/tagged",
                get(|| async { with_rule(StatusCode::IM_A_TEAPOT.into_response(), "test_rule") }),
            )
            .route("/untagged", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(super::log_request))
    }

    #[tokio::test]
    async fn passes_through_a_tagged_response_unchanged() {
        let request = Request::builder()
            .uri("/tagged")
            .body(Body::empty())
            .unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
    }

    #[tokio::test]
    async fn works_without_a_connect_info_extension() {
        let request = Request::builder()
            .uri("/untagged")
            .body(Body::empty())
            .unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
