//! JS challenge: a "checking your browser" gate. A request without the
//! validation cookie gets an HTML page whose deferred script waits
//! `delay_ms` before setting the cookie and reloading — a crawler that
//! doesn't execute JavaScript never gets past it, one that does passes
//! after the delay. Simulated: the cookie's value is fixed, not a real
//! cryptographic proof, since the point is testing whether the crawler
//! runs JavaScript and persists cookies, not solving a puzzle.

use axum::http::HeaderMap;
use axum::http::header::COOKIE;
use serde::Deserialize;
use serde::Serialize;

/// Name of the cookie a solved challenge sets.
pub(crate) const COOKIE_NAME: &str = "botcamp_challenge";

/// Value a solved challenge's cookie must carry.
pub(crate) const COOKIE_VALUE: &str = "ok";

/// Runtime-configurable challenge policy.
#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct ChallengeConfig {
    /// Delay, in milliseconds, before the challenge page's script sets
    /// the validation cookie and reloads.
    pub(crate) delay_ms: u64,

    /// `max-age`, in seconds, set on the validation cookie once solved.
    pub(crate) cookie_max_age_secs: u64,
}

impl Default for ChallengeConfig {
    fn default() -> Self {
        ChallengeConfig {
            delay_ms: 1500,
            cookie_max_age_secs: 3600,
        }
    }
}

/// Shared challenge state: just the current configuration — there's no
/// per-key state to track, since passing the gate only depends on
/// whether the request carries a valid cookie.
pub(crate) struct ChallengeState {
    config: tokio::sync::RwLock<ChallengeConfig>,
}

impl Default for ChallengeState {
    fn default() -> Self {
        ChallengeState {
            config: tokio::sync::RwLock::new(ChallengeConfig::default()),
        }
    }
}

impl ChallengeState {
    /// Returns a clone of the current configuration.
    pub(crate) async fn config(&self) -> ChallengeConfig {
        self.config.read().await.clone()
    }

    /// Replaces the current configuration.
    pub(crate) async fn configure(&self, config: ChallengeConfig) {
        *self.config.write().await = config;
    }
}

/// Whether `headers` carries a valid, already-solved challenge cookie.
pub(crate) fn is_solved(headers: &HeaderMap) -> bool {
    let Some(cookie) = headers.get(COOKIE).and_then(|value| value.to_str().ok()) else {
        return false;
    };

    cookie.split(';').any(|pair| {
        let mut parts = pair.trim().splitn(2, '=');

        matches!(
            (parts.next(), parts.next()),
            (Some(COOKIE_NAME), Some(COOKIE_VALUE))
        )
    })
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;
    use axum::http::HeaderValue;

    use super::ChallengeConfig;
    use super::ChallengeState;
    use super::is_solved;

    #[test]
    fn default_config_has_a_nonzero_delay() {
        let config = ChallengeConfig::default();

        assert!(config.delay_ms > 0);
    }

    #[tokio::test]
    async fn configure_replaces_the_policy() {
        let state = ChallengeState::default();

        state
            .configure(ChallengeConfig {
                delay_ms: 42,
                cookie_max_age_secs: 7,
            })
            .await;

        assert_eq!(state.config().await.delay_ms, 42);
    }

    #[test]
    fn is_solved_is_false_without_a_cookie_header() {
        assert!(!is_solved(&HeaderMap::new()));
    }

    #[test]
    fn is_solved_is_true_with_the_expected_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_static("other=1; botcamp_challenge=ok"),
        );

        assert!(is_solved(&headers));
    }

    #[test]
    fn is_solved_is_false_with_the_wrong_cookie_value() {
        let mut headers = HeaderMap::new();
        headers.insert("cookie", HeaderValue::from_static("botcamp_challenge=no"));

        assert!(!is_solved(&headers));
    }
}
