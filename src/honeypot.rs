//! Honeypot: an instant, unconditional ban for any client that reaches
//! `/honeypot/*` — a well-behaved crawler should never fetch anything
//! there, since the only way in is a link hidden from real users. Kept
//! entirely separate from [`crate::rate_limit`]: a different violation,
//! a different store, its own config/reset/status.

use std::time::Duration;
use std::time::Instant;

use dashmap::DashMap;
use serde::Deserialize;

use crate::rate_limit::KeyStrategy;

/// Runtime-configurable honeypot policy.
#[derive(Clone, Deserialize)]
pub(crate) struct HoneypotConfig {
    /// How a client is identified: by IP, `User-Agent`, or both.
    pub(crate) key_strategy: KeyStrategy,

    /// Ban duration, in milliseconds, once a key reaches any path under
    /// `/honeypot/`.
    pub(crate) ban_duration_ms: u64,
}

impl Default for HoneypotConfig {
    fn default() -> Self {
        HoneypotConfig {
            key_strategy: KeyStrategy::Ip,
            ban_duration_ms: 600_000,
        }
    }
}

/// Introspection data for a single key, as returned by
/// [`HoneypotState::status`].
pub(crate) struct KeyStatus {
    /// Whether the key is currently banned.
    pub(crate) banned: bool,

    /// Seconds remaining on the ban, if any.
    pub(crate) retry_after_secs: Option<u64>,
}

/// Shared honeypot state: the current configuration and every caught
/// key's ban expiry.
pub(crate) struct HoneypotState {
    config: tokio::sync::RwLock<HoneypotConfig>,
    banned: DashMap<String, Instant>,
}

impl Default for HoneypotState {
    fn default() -> Self {
        HoneypotState {
            config: tokio::sync::RwLock::new(HoneypotConfig::default()),
            banned: DashMap::new(),
        }
    }
}

impl HoneypotState {
    /// Returns a clone of the current configuration.
    pub(crate) async fn config(&self) -> HoneypotConfig {
        self.config.read().await.clone()
    }

    /// Replaces the current configuration and clears every ban.
    pub(crate) async fn configure(&self, config: HoneypotConfig) {
        *self.config.write().await = config;
        self.banned.clear();
    }

    /// Clears every ban, without changing the configuration.
    pub(crate) fn reset(&self) {
        self.banned.clear();
    }

    /// Unconditionally bans `key` for the configured `ban_duration_ms` —
    /// sprung by reaching any path under `/honeypot/`.
    pub(crate) async fn spring(&self, key: &str) {
        let ban_duration_ms = self.config.read().await.ban_duration_ms;
        let banned_until = Instant::now() + Duration::from_millis(ban_duration_ms);
        self.banned.insert(key.to_string(), banned_until);
    }

    /// Returns `Some(retry_after_secs)` if `key` is currently banned,
    /// `None` otherwise.
    pub(crate) fn retry_after_secs(&self, key: &str) -> Option<u64> {
        let banned_until = *self.banned.get(key)?;
        let now = Instant::now();

        if now < banned_until {
            Some((banned_until - now).as_secs().max(1))
        } else {
            None
        }
    }

    /// Returns introspection data for `key`.
    pub(crate) fn status(&self, key: &str) -> KeyStatus {
        match self.retry_after_secs(key) {
            Some(retry_after_secs) => KeyStatus {
                banned: true,
                retry_after_secs: Some(retry_after_secs),
            },
            None => KeyStatus {
                banned: false,
                retry_after_secs: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::HoneypotConfig;
    use super::HoneypotState;
    use crate::rate_limit::KeyStrategy;

    fn state_with(ban_duration_ms: u64) -> HoneypotState {
        HoneypotState {
            config: tokio::sync::RwLock::new(HoneypotConfig {
                key_strategy: KeyStrategy::Ip,
                ban_duration_ms,
            }),
            banned: dashmap::DashMap::new(),
        }
    }

    #[tokio::test]
    async fn a_key_is_not_banned_until_it_springs_the_trap() {
        let state = state_with(1000);

        assert!(state.retry_after_secs("a").is_none());
    }

    #[tokio::test]
    async fn springing_the_trap_bans_the_key() {
        let state = state_with(1000);

        state.spring("a").await;

        assert!(state.retry_after_secs("a").is_some());
    }

    #[tokio::test]
    async fn a_ban_expires_after_its_configured_duration() {
        let state = state_with(50);

        state.spring("a").await;
        assert!(state.retry_after_secs("a").is_some());

        tokio::time::sleep(Duration::from_millis(80)).await;

        assert!(state.retry_after_secs("a").is_none());
    }

    #[tokio::test]
    async fn other_keys_are_unaffected() {
        let state = state_with(1000);

        state.spring("a").await;

        assert!(state.retry_after_secs("b").is_none());
    }

    #[tokio::test]
    async fn reset_clears_every_ban() {
        let state = state_with(1000);

        state.spring("a").await;
        state.reset();

        assert!(state.retry_after_secs("a").is_none());
    }

    #[tokio::test]
    async fn configure_replaces_the_policy_and_clears_bans() {
        let state = state_with(1000);
        state.spring("a").await;

        state
            .configure(HoneypotConfig {
                key_strategy: KeyStrategy::Ip,
                ban_duration_ms: 2000,
            })
            .await;

        assert!(state.retry_after_secs("a").is_none());
    }

    #[test]
    fn status_reports_unbanned_for_an_unknown_key() {
        let state = HoneypotState::default();

        let status = state.status("unknown");

        assert!(!status.banned);
        assert!(status.retry_after_secs.is_none());
    }
}
