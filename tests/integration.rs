//! Integration tests for the bot-camp server.

use std::time::Instant;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

use bot_camp::app;

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
