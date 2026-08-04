//! Integration tests for the bot-camp server.

use std::net::SocketAddr;
use std::time::Instant;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::Request;
use axum::http::StatusCode;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use tower::ServiceExt;

use bot_camp::app;

/// Builds a request carrying a `ConnectInfo<SocketAddr>` extension, the
/// way axum's real connection-serving layer would — needed for any
/// route reached through the rate limiting middleware, since it keys on
/// the peer address.
fn request_from(peer: &str, uri: &str) -> Request<Body> {
    let mut request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let addr: SocketAddr = peer.parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(addr));
    request
}

#[tokio::test]
async fn auth_basic_accepts_the_expected_credentials() {
    // Build request: "bot-camp:bot-camp" base64-encoded
    let request = Request::builder()
        .uri("/auth/basic")
        .header("Authorization", "Basic Ym90LWNhbXA6Ym90LWNhbXA=")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_basic_challenges_a_missing_authorization_header() {
    // Build request
    let request = Request::builder()
        .uri("/auth/basic")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Verify the challenge header is present
    assert!(response.headers().contains_key("www-authenticate"));
}

#[tokio::test]
async fn auth_basic_rejects_wrong_credentials() {
    // Build request: "bot-camp:wrong" base64-encoded
    let request = Request::builder()
        .uri("/auth/basic")
        .header("Authorization", "Basic Ym90LWNhbXA6d3Jvbmc=")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

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

#[tokio::test]
async fn large_response_returns_a_body_of_the_requested_size() {
    // Build request
    let request = Request::builder()
        .uri("/large-response/2")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.len(), 2048);
}

#[tokio::test]
async fn delay_waits_before_responding() {
    // Build request
    let request = Request::builder()
        .uri("/delay/20")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let start = Instant::now();
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the delay was actually observed
    assert!(start.elapsed().as_millis() >= 20);

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body.is_empty());
}

#[tokio::test]
async fn redirect_returns_the_requested_code_and_location() {
    // Build request
    let request = Request::builder()
        .uri("/redirect/301?to=/status/200")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);

    // Verify location
    assert_eq!(response.headers().get("location").unwrap(), "/status/200");
}

#[tokio::test]
async fn redirect_rejects_a_non_redirect_code() {
    // Build request: 200 isn't a redirect status
    let request = Request::builder()
        .uri("/redirect/200?to=/status/200")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn redirect_chain_decrements_n_until_it_reaches_to() {
    // Build request
    let request = Request::builder()
        .uri("/redirect/chain?n=2&to=/status/200")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::FOUND);

    // Verify location points at the next hop
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/redirect/chain?n=1&to=/status/200"
    );
}

#[tokio::test]
async fn redirect_loop_cycles_through_its_positions() {
    // Build request
    let request = Request::builder()
        .uri("/redirect/loop?steps=2&step=1")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::FOUND);

    // Verify location wraps back to the first position
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/redirect/loop?steps=2&step=0"
    );
}

#[tokio::test]
async fn redirect_refresh_sets_the_refresh_header() {
    // Build request
    let request = Request::builder()
        .uri("/redirect/refresh?delay=5&to=/status/200")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the refresh header
    assert_eq!(
        response.headers().get("refresh").unwrap(),
        "5; url=/status/200"
    );
}

#[tokio::test]
async fn redirect_meta_refresh_embeds_the_expected_tag() {
    // Build request
    let request = Request::builder()
        .uri("/redirect/meta-refresh?delay=5&to=/status/200")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#"<meta http-equiv="refresh" content="5; url=/status/200">"#));
}

#[tokio::test]
async fn normalize_redirects_to_the_normalized_form() {
    // Build request
    let request = Request::builder()
        .uri("/normalize?url=HTTP://ExAmPle.COM:80/a/./b/../c/?d=2%26c=1%23frag")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);

    // Verify the location points at the fully normalized URL
    assert_eq!(
        response.headers().get("location").unwrap(),
        "http://example.com/a/c?c=1&d=2"
    );
}

#[tokio::test]
async fn normalize_returns_ok_when_the_url_is_already_normalized() {
    // Build request
    let request = Request::builder()
        .uri("/normalize?url=http://example.com/path")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify no redirect was issued
    assert!(response.headers().get("location").is_none());
}

#[tokio::test]
async fn normalize_can_disable_query_sorting() {
    // Build request: already normalized except for query order, and
    // sort_query is turned off, so nothing should change
    let request = Request::builder()
        .uri("/normalize?url=http://example.com/path?c=3%26a=1%26b=2&sort_query=false")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify no redirect was issued
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("location").is_none());
}

#[tokio::test]
async fn normalize_rejects_an_unparseable_url() {
    // Build request
    let request = Request::builder()
        .uri("/normalize?url=not+a+url")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn robots_txt_serves_the_default_content_initially() {
    // Build request
    let request = Request::builder()
        .uri("/robots.txt")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body, "User-agent: *\nAllow: /\n");
}

#[tokio::test]
async fn robots_txt_reflects_a_prior_put() {
    // A single router instance, so its in-memory state is shared across
    // both requests below
    let router = app();

    // Build the PUT request
    let put_request = Request::builder()
        .method("PUT")
        .uri("/robots.txt")
        .body(Body::from("User-agent: Googlebot\nDisallow: /private\n"))
        .unwrap();

    // Send the PUT request
    let put_response = router.clone().oneshot(put_request).await.unwrap();

    // Verify status
    assert_eq!(put_response.status(), StatusCode::OK);

    // Build the follow-up GET request
    let get_request = Request::builder()
        .uri("/robots.txt")
        .body(Body::empty())
        .unwrap();

    // Send the GET request
    let get_response = router.oneshot(get_request).await.unwrap();

    // Verify the GET reflects what was PUT
    let body = get_response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body, "User-agent: Googlebot\nDisallow: /private\n");
}

#[tokio::test]
async fn robots_meta_renders_the_directives_and_sets_the_conflicting_header() {
    // Build request
    let request = Request::builder()
        .uri("/robots/meta?directives=index&x_robots_tag=noindex")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the conflicting header
    assert_eq!(response.headers().get("x-robots-tag").unwrap(), "noindex");

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#"<meta name="robots" content="index">"#));
}

#[tokio::test]
async fn content_omits_title_and_h1_by_default() {
    // Build request
    let request = Request::builder()
        .uri("/content")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(!body.contains("<title>"));
    assert!(!body.contains("<h1>"));
}

#[tokio::test]
async fn content_renders_title_h1_and_word_count() {
    // Build request
    let request = Request::builder()
        .uri("/content?title=Page&h1=Heading&word_count=3")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("<title>Page</title>"));
    assert!(body.contains("<h1>Heading</h1>"));
    assert!(body.contains("word0 word1 word2"));
}

#[tokio::test]
async fn content_can_serve_the_same_body_from_two_different_urls() {
    // Build requests: same `body`, different paths, to simulate
    // duplicate content across two pages
    let request_a = Request::builder()
        .uri("/content?body=shared+text&title=A")
        .body(Body::empty())
        .unwrap();
    let request_b = Request::builder()
        .uri("/content?body=shared+text&title=B")
        .body(Body::empty())
        .unwrap();

    // Send requests to app
    let response_a = app().oneshot(request_a).await.unwrap();
    let response_b = app().oneshot(request_b).await.unwrap();

    // Verify both pages share the same body text
    let body_a = response_a.into_body().collect().await.unwrap().to_bytes();
    let body_a = String::from_utf8(body_a.to_vec()).unwrap();
    let body_b = response_b.into_body().collect().await.unwrap().to_bytes();
    let body_b = String::from_utf8(body_b.to_vec()).unwrap();
    assert!(body_a.contains("shared text"));
    assert!(body_b.contains("shared text"));
}

#[tokio::test]
async fn js_render_omits_every_signal_from_the_initial_html() {
    // Build request
    let request = Request::builder()
        .uri("/js-render?text=hello&title=Injected&canonical=/page&delay_ms=500")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the initial HTML carries none of the injected signals
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(!body.contains("<title>"));
    assert!(!body.contains(r#"rel="canonical""#));
}

#[tokio::test]
async fn js_render_embeds_the_requested_injections_in_a_deferred_script() {
    // Build request
    let request = Request::builder()
        .uri("/js-render?text=hello&title=Injected&canonical=/page&delay_ms=500")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("document.getElementById('js-content').textContent = \"hello\";"));
    assert!(body.contains("document.title = \"Injected\";"));
    assert!(body.contains("link.href = \"/page\";"));
    assert!(body.contains("}, 500);"));
}

#[tokio::test]
async fn encoding_declares_the_requested_content_type_charset() {
    // Build request: header says iso-8859-1, meta tag says utf-8
    let request = Request::builder()
        .uri("/encoding?content_type_charset=iso-8859-1&meta_charset=utf-8")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the mismatched charset declarations
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/html; charset=iso-8859-1"
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#"<meta charset="utf-8">"#));
}

#[tokio::test]
async fn encoding_double_encodes_the_body_when_requested() {
    // Build request
    let request = Request::builder()
        .uri("/encoding?text=a%26b&double_encode=true")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify the ampersand was HTML-entity-encoded twice
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("a&amp;amp;b"));
}

#[tokio::test]
async fn broken_html_splices_raw_markup_into_head_and_body() {
    // Build request
    let request = Request::builder()
        .uri("/broken-html?head=%3Cp%3Ebad%3C%2Fp%3E&body=%3Clink+rel%3D%22x%22%3E")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify both raw snippets were spliced in, unescaped
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    let head_end = body.find("</head>").unwrap();
    assert!(body[..head_end].contains("<p>bad</p>"));
    assert!(body[head_end..].contains(r#"<link rel="x">"#));
}

/// Configures the rate limiter with a token bucket of `capacity` and a
/// negligible refill rate, so a second request from the same key is
/// reliably limited without depending on real timing.
fn configure_request(capacity: u32, ban_threshold: u32, key_strategy: &str) -> Request<Body> {
    let body = format!(
        r#"{{"algorithm":"token_bucket","capacity":{capacity},"refill_per_sec":0.0001,
            "key_strategy":"{key_strategy}","ban_threshold":{ban_threshold},"ban_duration_ms":100}}"#
    );

    Request::builder()
        .method("PUT")
        .uri("/ratelimit/config")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn ratelimit_allows_requests_within_capacity_and_limits_beyond_it() {
    let router = app();

    let configure = router
        .clone()
        .oneshot(configure_request(1, 10, "ip"))
        .await
        .unwrap();
    assert_eq!(configure.status(), StatusCode::OK);

    let first = router
        .clone()
        .oneshot(request_from("203.0.113.1:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let second = router
        .oneshot(request_from("203.0.113.1:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(second.headers().get("retry-after").is_some());
}

#[tokio::test]
async fn ratelimit_bans_after_reaching_the_violation_threshold() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request(1, 1, "ip"))
        .await
        .unwrap();

    // First request consumes the only token; second violates the limit
    // and immediately reaches the ban threshold of 1
    router
        .clone()
        .oneshot(request_from("203.0.113.2:1", "/ratelimit/page"))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("203.0.113.2:1", "/ratelimit/page"))
        .await
        .unwrap();

    let banned = router
        .oneshot(request_from("203.0.113.2:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(banned.status(), StatusCode::FORBIDDEN);
    assert!(banned.headers().get("retry-after").is_some());
}

#[tokio::test]
async fn ratelimit_reset_clears_counters() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request(1, 10, "ip"))
        .await
        .unwrap();

    router
        .clone()
        .oneshot(request_from("203.0.113.3:1", "/ratelimit/page"))
        .await
        .unwrap();
    let limited = router
        .clone()
        .oneshot(request_from("203.0.113.3:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);

    let reset_request = Request::builder()
        .method("POST")
        .uri("/ratelimit/reset")
        .body(Body::empty())
        .unwrap();
    let reset_response = router.clone().oneshot(reset_request).await.unwrap();
    assert_eq!(reset_response.status(), StatusCode::OK);

    let allowed_again = router
        .oneshot(request_from("203.0.113.3:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(allowed_again.status(), StatusCode::OK);
}

#[tokio::test]
async fn ratelimit_admin_endpoints_stay_reachable_while_banned() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request(1, 1, "ip"))
        .await
        .unwrap();

    // Exhaust the quota and reach the ban
    router
        .clone()
        .oneshot(request_from("203.0.113.4:1", "/ratelimit/page"))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("203.0.113.4:1", "/ratelimit/page"))
        .await
        .unwrap();
    let banned = router
        .clone()
        .oneshot(request_from("203.0.113.4:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(banned.status(), StatusCode::FORBIDDEN);

    // The admin endpoints are never gated by the same limiter
    let status_request = request_from("203.0.113.4:1", "/ratelimit/status");
    let status_response = router.clone().oneshot(status_request).await.unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);

    let reset_request = Request::builder()
        .method("POST")
        .uri("/ratelimit/reset")
        .body(Body::empty())
        .unwrap();
    let reset_response = router.oneshot(reset_request).await.unwrap();
    assert_eq!(reset_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ratelimit_does_not_affect_pre_existing_routes() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request(1, 1, "ip"))
        .await
        .unwrap();

    // Exhaust the quota and reach the ban on the playground
    router
        .clone()
        .oneshot(request_from("203.0.113.5:1", "/ratelimit/page"))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("203.0.113.5:1", "/ratelimit/page"))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("203.0.113.5:1", "/ratelimit/page"))
        .await
        .unwrap();

    // An unrelated, pre-existing route from the same peer is unaffected
    let request = request_from("203.0.113.5:1", "/status/200");
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ratelimit_status_reports_the_ban_state() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request(1, 1, "ip"))
        .await
        .unwrap();

    router
        .clone()
        .oneshot(request_from("203.0.113.6:1", "/ratelimit/page"))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("203.0.113.6:1", "/ratelimit/page"))
        .await
        .unwrap();

    let status_request = request_from("203.0.113.6:1", "/ratelimit/status");
    let response = router.oneshot(status_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#""banned":true"#));
}

#[tokio::test]
async fn ratelimit_can_key_by_user_agent_instead_of_ip() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request(1, 10, "user_agent"))
        .await
        .unwrap();

    // Same User-Agent, different peer IPs: still share one quota
    let mut first = request_from("203.0.113.7:1", "/ratelimit/page");
    first
        .headers_mut()
        .insert("user-agent", "shared-bot".parse().unwrap());
    let first_response = router.clone().oneshot(first).await.unwrap();
    assert_eq!(first_response.status(), StatusCode::OK);

    let mut second = request_from("203.0.113.8:1", "/ratelimit/page");
    second
        .headers_mut()
        .insert("user-agent", "shared-bot".parse().unwrap());
    let second_response = router.oneshot(second).await.unwrap();
    assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// Builds a `PUT /ratelimit/config` request with a capacity-1 token
/// bucket (so a second request from a non-listed key would normally be
/// limited), plus the given `block_ips`/`allow_ips` entries.
fn configure_request_with_lists(block_ips: &str, allow_ips: &str) -> Request<Body> {
    let body = format!(
        r#"{{"algorithm":"token_bucket","capacity":1,"refill_per_sec":0.0001,
            "key_strategy":"ip","ban_threshold":10,"ban_duration_ms":100,
            "block_ips":[{block_ips}],"allow_ips":[{allow_ips}]}}"#
    );

    Request::builder()
        .method("PUT")
        .uri("/ratelimit/config")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn ratelimit_block_list_rejects_a_matching_ip_immediately() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request_with_lists(r#""203.0.113.20""#, ""))
        .await
        .unwrap();

    let response = router
        .oneshot(request_from("203.0.113.20:1", "/ratelimit/page"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn ratelimit_allow_list_bypasses_the_algorithm_entirely() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request_with_lists("", r#""203.0.113.21""#))
        .await
        .unwrap();

    // Capacity is 1, but the allow-listed IP should never be limited,
    // no matter how many requests it makes
    for _ in 0..5 {
        let response = router
            .clone()
            .oneshot(request_from("203.0.113.21:1", "/ratelimit/page"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn ratelimit_block_list_wins_over_allow_list() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request_with_lists(
            r#""203.0.113.22""#,
            r#""203.0.113.22""#,
        ))
        .await
        .unwrap();

    let response = router
        .oneshot(request_from("203.0.113.22:1", "/ratelimit/page"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn ratelimit_status_reports_block_and_allow_list_membership() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request_with_lists(
            r#""203.0.113.23""#,
            r#""203.0.113.24""#,
        ))
        .await
        .unwrap();

    let blocked_status = router
        .clone()
        .oneshot(request_from("203.0.113.23:1", "/ratelimit/status"))
        .await
        .unwrap();
    let body = blocked_status
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#""blocked":true"#));

    let allowed_status = router
        .oneshot(request_from("203.0.113.24:1", "/ratelimit/status"))
        .await
        .unwrap();
    let body = allowed_status
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#""allow_listed":true"#));
}

#[tokio::test]
async fn ratelimit_min_interval_limits_a_request_that_arrives_too_soon() {
    let router = app();

    let body = r#"{"algorithm":"min_interval","min_interval_ms":60000,
        "key_strategy":"ip","ban_threshold":10,"ban_duration_ms":100}"#;
    let configure = Request::builder()
        .method("PUT")
        .uri("/ratelimit/config")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    router.clone().oneshot(configure).await.unwrap();

    let first = router
        .clone()
        .oneshot(request_from("203.0.113.30:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let second = router
        .oneshot(request_from("203.0.113.30:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(second.headers().get("retry-after").is_some());
}

fn honeypot_configure_request(ban_duration_ms: u64) -> Request<Body> {
    let body = format!(r#"{{"key_strategy":"ip","ban_duration_ms":{ban_duration_ms}}}"#);

    Request::builder()
        .method("PUT")
        .uri("/honeypot/config")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn honeypot_allows_the_first_visit_and_bans_afterwards() {
    let router = app();

    router
        .clone()
        .oneshot(honeypot_configure_request(60_000))
        .await
        .unwrap();

    let first = router
        .clone()
        .oneshot(request_from("198.51.100.1:1", "/honeypot/trap"))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let second = router
        .oneshot(request_from("198.51.100.1:1", "/honeypot/trap"))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::FORBIDDEN);
    assert!(second.headers().get("retry-after").is_some());
}

#[tokio::test]
async fn honeypot_bans_apply_to_any_path_under_the_prefix() {
    let router = app();

    router
        .clone()
        .oneshot(honeypot_configure_request(60_000))
        .await
        .unwrap();

    router
        .clone()
        .oneshot(request_from("198.51.100.2:1", "/honeypot/some/deep/path"))
        .await
        .unwrap();

    let other_path = router
        .oneshot(request_from("198.51.100.2:1", "/honeypot/another-path"))
        .await
        .unwrap();
    assert_eq!(other_path.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn honeypot_reset_clears_bans() {
    let router = app();

    router
        .clone()
        .oneshot(honeypot_configure_request(60_000))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("198.51.100.3:1", "/honeypot/trap"))
        .await
        .unwrap();

    let reset_request = Request::builder()
        .method("POST")
        .uri("/honeypot/reset")
        .body(Body::empty())
        .unwrap();
    let reset_response = router.clone().oneshot(reset_request).await.unwrap();
    assert_eq!(reset_response.status(), StatusCode::OK);

    let allowed_again = router
        .oneshot(request_from("198.51.100.3:1", "/honeypot/trap"))
        .await
        .unwrap();
    assert_eq!(allowed_again.status(), StatusCode::OK);
}

#[tokio::test]
async fn honeypot_admin_endpoints_stay_reachable_while_banned() {
    let router = app();

    router
        .clone()
        .oneshot(honeypot_configure_request(60_000))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("198.51.100.4:1", "/honeypot/trap"))
        .await
        .unwrap();

    let status_request = request_from("198.51.100.4:1", "/honeypot/status");
    let status_response = router.clone().oneshot(status_request).await.unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let body = status_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#""banned":true"#));

    let reset_request = Request::builder()
        .method("POST")
        .uri("/honeypot/reset")
        .body(Body::empty())
        .unwrap();
    let reset_response = router.oneshot(reset_request).await.unwrap();
    assert_eq!(reset_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn honeypot_ban_does_not_affect_pre_existing_routes_or_the_ratelimit_playground() {
    let router = app();

    router
        .clone()
        .oneshot(honeypot_configure_request(60_000))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("198.51.100.5:1", "/honeypot/trap"))
        .await
        .unwrap();

    let status_response = router
        .clone()
        .oneshot(request_from("198.51.100.5:1", "/status/200"))
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);

    let ratelimit_response = router
        .oneshot(request_from("198.51.100.5:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(ratelimit_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn challenge_serves_the_checking_page_without_the_cookie() {
    let request = Request::builder()
        .uri("/challenge/page")
        .body(Body::empty())
        .unwrap();

    let response = app().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("botcamp_challenge=ok"));
    assert!(!body.contains("ok: /challenge/page"));
}

#[tokio::test]
async fn challenge_lets_a_request_with_the_valid_cookie_through() {
    let request = Request::builder()
        .uri("/challenge/page")
        .header("cookie", "botcamp_challenge=ok")
        .body(Body::empty())
        .unwrap();

    let response = app().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body, "ok: /challenge/page");
}

#[tokio::test]
async fn challenge_config_changes_the_rendered_delay() {
    let router = app();

    let configure = Request::builder()
        .method("PUT")
        .uri("/challenge/config")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"delay_ms":9000,"cookie_max_age_secs":30}"#))
        .unwrap();
    let configure_response = router.clone().oneshot(configure).await.unwrap();
    assert_eq!(configure_response.status(), StatusCode::OK);

    let request = Request::builder()
        .uri("/challenge/page")
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("}, 9000);"));
    assert!(body.contains("max-age=30"));
}

#[tokio::test]
async fn content_renders_a_hidden_link_pointing_at_the_honeypot() {
    let request = Request::builder()
        .uri("/content?hidden_link=/honeypot/trap")
        .body(Body::empty())
        .unwrap();

    let response = app().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#"href="/honeypot/trap""#));
}

#[tokio::test]
async fn canonical_renders_a_self_referential_link_by_default() {
    // Build request
    let request = Request::builder()
        .uri("/canonical?to=/page")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify status
    assert_eq!(response.status(), StatusCode::OK);

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body.matches(r#"href="/page""#).count(), 1);
}

#[tokio::test]
async fn canonical_can_duplicate_and_move_the_tag_into_the_body() {
    // Build request
    let request = Request::builder()
        .uri("/canonical?to=/page&duplicate=true&in_body=true")
        .body(Body::empty())
        .unwrap();

    // Send request to app
    let response = app().oneshot(request).await.unwrap();

    // Verify body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    let head_end = body.find("</head>").unwrap();

    // Verify no canonical link ended up in the head
    assert!(!body[..head_end].contains(r#"href="/page""#));

    // Verify both duplicated links ended up in the body
    assert_eq!(body[head_end..].matches(r#"href="/page""#).count(), 2);
}

#[tokio::test]
async fn dashboard_index_serves_the_embedded_page() {
    let request = Request::builder()
        .uri("/dashboard")
        .body(Body::empty())
        .unwrap();

    let response = app().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/html");

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("<!doctype html>"));
}

#[tokio::test]
async fn dashboard_assets_returns_404_for_an_unknown_path() {
    let request = Request::builder()
        .uri("/dashboard/does-not-exist.js")
        .body(Body::empty())
        .unwrap();

    let response = app().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dashboard_snapshot_reflects_a_key_banned_by_a_prior_request() {
    let router = app();

    router
        .clone()
        .oneshot(configure_request(1, 1, "ip"))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("203.0.113.40:1", "/ratelimit/page"))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("203.0.113.40:1", "/ratelimit/page"))
        .await
        .unwrap();

    let request = Request::builder()
        .uri("/dashboard/snapshot")
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains(r#""key":"203.0.113.40""#));
    assert!(body.contains(r#""banned":true"#));
}

#[tokio::test]
async fn dashboard_ws_streams_the_snapshot_then_a_live_event() {
    let router = app();

    // A real socket is unavoidable here: a WebSocket upgrade needs an
    // actual HTTP connection to hand off, which `oneshot` can't provide.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let serve_router = router.clone();
    tokio::spawn(async move {
        axum::serve(listener, serve_router.into_make_service())
            .await
            .unwrap();
    });

    let (mut ws_stream, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/dashboard/ws"))
        .await
        .unwrap();

    let snapshot = ws_stream.next().await.unwrap().unwrap();
    assert!(
        snapshot
            .into_text()
            .unwrap()
            .contains(r#""type":"snapshot""#)
    );

    // Ban a key through the very same (Arc-shared) state, via an
    // in-process request rather than another real connection. Capacity 1
    // with a ban threshold of 1 means the first request is allowed (it
    // consumes the only token) and the second is what bans the key — two
    // events, in that order.
    router
        .clone()
        .oneshot(configure_request(1, 1, "ip"))
        .await
        .unwrap();
    router
        .clone()
        .oneshot(request_from("203.0.113.41:1", "/ratelimit/page"))
        .await
        .unwrap();
    let banned = router
        .oneshot(request_from("203.0.113.41:1", "/ratelimit/page"))
        .await
        .unwrap();
    assert_eq!(banned.status(), StatusCode::FORBIDDEN);

    let allowed_event = ws_stream.next().await.unwrap().unwrap();
    assert!(
        allowed_event
            .into_text()
            .unwrap()
            .contains(r#""decision":"allowed""#)
    );

    let banned_event = ws_stream.next().await.unwrap().unwrap();
    let text = banned_event.into_text().unwrap();
    assert!(text.contains(r#""type":"event""#));
    assert!(text.contains(r#""decision":"banned""#));
    assert!(text.contains(r#""key":"203.0.113.41""#));
}
