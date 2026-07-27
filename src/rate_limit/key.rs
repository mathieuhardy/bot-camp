//! Client identification for rate limiting: by IP, User-Agent, or both.

use std::net::SocketAddr;

use axum::http::HeaderMap;
use serde::Deserialize;

/// How a client is identified for rate limiting purposes.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KeyStrategy {
    Ip,
    UserAgent,
    Both,
}

/// Derives the rate limiting key for a request under `strategy`: the
/// client's IP, its `User-Agent`, or both.
///
/// The IP trusts `X-Forwarded-For`'s first value if present, since
/// bot-camp may run behind a reverse proxy, falling back to `peer` (the
/// real TCP peer address) otherwise.
pub(crate) fn extract_key(strategy: KeyStrategy, headers: &HeaderMap, peer: SocketAddr) -> String {
    match strategy {
        KeyStrategy::Ip => client_ip(headers, peer),
        KeyStrategy::UserAgent => user_agent(headers),
        KeyStrategy::Both => format!("{}|{}", client_ip(headers, peer), user_agent(headers)),
    }
}

/// Extracts the client IP from `X-Forwarded-For`'s first value, or
/// `peer` if the header is absent or unparseable.
fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| peer.ip().to_string())
}

/// Extracts the client's `User-Agent`, or `"unknown"` if absent.
fn user_agent(headers: &HeaderMap) -> String {
    headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;
    use axum::http::HeaderValue;

    use super::KeyStrategy;
    use super::extract_key;

    fn peer() -> std::net::SocketAddr {
        "203.0.113.1:12345".parse().unwrap()
    }

    #[test]
    fn ip_strategy_prefers_x_forwarded_for_over_the_peer_address() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("198.51.100.7, 10.0.0.1"),
        );

        let key = extract_key(KeyStrategy::Ip, &headers, peer());

        assert_eq!(key, "198.51.100.7");
    }

    #[test]
    fn ip_strategy_falls_back_to_the_peer_address() {
        let key = extract_key(KeyStrategy::Ip, &HeaderMap::new(), peer());

        assert_eq!(key, "203.0.113.1");
    }

    #[test]
    fn user_agent_strategy_reads_the_header() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("test-bot/1.0"));

        let key = extract_key(KeyStrategy::UserAgent, &headers, peer());

        assert_eq!(key, "test-bot/1.0");
    }

    #[test]
    fn both_strategy_combines_ip_and_user_agent() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("test-bot/1.0"));

        let key = extract_key(KeyStrategy::Both, &headers, peer());

        assert_eq!(key, "203.0.113.1|test-bot/1.0");
    }
}
