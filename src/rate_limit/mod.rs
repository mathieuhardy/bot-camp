//! Rate limiting engine: pluggable algorithms, per-key state, and a
//! two-tier ban (temporary block after repeated violations).

mod algorithm;
mod key;

pub(crate) use algorithm::Algorithm;
pub(crate) use key::KeyStrategy;
pub(crate) use key::extract_key;

use std::time::Duration;
use std::time::Instant;

use dashmap::DashMap;
use serde::Deserialize;

use algorithm::AlgorithmDecision;
use algorithm::AlgorithmState;

/// Runtime-configurable rate limiting policy.
#[derive(Clone, Deserialize)]
pub(crate) struct RateLimitConfig {
    /// The algorithm to enforce, and its parameters.
    #[serde(flatten)]
    pub(crate) algorithm: Algorithm,

    /// How a client is identified: by IP, `User-Agent`, or both.
    pub(crate) key_strategy: KeyStrategy,

    /// Consecutive violations before a key is temporarily banned.
    pub(crate) ban_threshold: u32,

    /// Ban duration, in milliseconds, once `ban_threshold` is reached.
    pub(crate) ban_duration_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            algorithm: Algorithm::TokenBucket {
                capacity: 10,
                refill_per_sec: 1.0,
            },
            key_strategy: KeyStrategy::Ip,
            ban_threshold: 3,
            ban_duration_ms: 300_000,
        }
    }
}

/// Outcome of a rate limit check for one request.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The request is allowed.
    Allowed,

    /// The request exceeds the configured algorithm's rate; retry after
    /// this many seconds.
    Limited { retry_after_secs: u64 },

    /// The key is temporarily banned after repeated violations; retry
    /// after this many seconds.
    Banned { retry_after_secs: u64 },
}

/// Introspection data for a single key, as returned by
/// [`RateLimitState::status`].
pub(crate) struct KeyStatus {
    /// Whether the key is currently banned.
    pub(crate) banned: bool,

    /// Seconds remaining on the ban, if any.
    pub(crate) retry_after_secs: Option<u64>,
}

/// Per-key state: the algorithm's own bookkeeping, plus violation/ban
/// tracking shared across every algorithm.
struct KeyState {
    algorithm_state: AlgorithmState,
    consecutive_violations: u32,
    banned_until: Option<Instant>,
}

/// Shared rate limiting state: the current configuration and every
/// key's counters.
pub(crate) struct RateLimitState {
    config: tokio::sync::RwLock<RateLimitConfig>,
    keys: DashMap<String, KeyState>,
}

impl Default for RateLimitState {
    fn default() -> Self {
        RateLimitState {
            config: tokio::sync::RwLock::new(RateLimitConfig::default()),
            keys: DashMap::new(),
        }
    }
}

impl RateLimitState {
    /// Returns a clone of the current configuration.
    pub(crate) async fn config(&self) -> RateLimitConfig {
        self.config.read().await.clone()
    }

    /// Replaces the current configuration and clears every key's state,
    /// since counters from the old algorithm don't carry meaning under
    /// the new one.
    pub(crate) async fn configure(&self, config: RateLimitConfig) {
        *self.config.write().await = config;
        self.keys.clear();
    }

    /// Clears every key's counters and bans, without changing the
    /// configuration.
    pub(crate) fn reset(&self) {
        self.keys.clear();
    }

    /// Evaluates `key` against the current configuration, mutating its
    /// state.
    pub(crate) async fn check(&self, key: &str) -> Decision {
        let config = self.config.read().await;
        let now = Instant::now();

        let mut entry = self
            .keys
            .entry(key.to_string())
            .or_insert_with(|| KeyState {
                algorithm_state: AlgorithmState::new(&config.algorithm, now),
                consecutive_violations: 0,
                banned_until: None,
            });

        if let Some(banned_until) = entry.banned_until {
            if now < banned_until {
                return Decision::Banned {
                    retry_after_secs: (banned_until - now).as_secs().max(1),
                };
            }

            entry.banned_until = None;
            entry.consecutive_violations = 0;
        }

        match entry.algorithm_state.check(&config.algorithm, now) {
            AlgorithmDecision::Allowed => {
                entry.consecutive_violations = 0;
                Decision::Allowed
            }

            AlgorithmDecision::Limited { retry_after_secs } => {
                entry.consecutive_violations += 1;

                if entry.consecutive_violations >= config.ban_threshold {
                    let ban_duration = Duration::from_millis(config.ban_duration_ms);
                    entry.banned_until = Some(now + ban_duration);

                    Decision::Banned {
                        retry_after_secs: ban_duration.as_secs().max(1),
                    }
                } else {
                    Decision::Limited { retry_after_secs }
                }
            }
        }
    }

    /// Returns introspection data for `key`, without mutating its state.
    pub(crate) fn status(&self, key: &str) -> KeyStatus {
        let Some(state) = self.keys.get(key) else {
            return KeyStatus {
                banned: false,
                retry_after_secs: None,
            };
        };

        let Some(banned_until) = state.banned_until else {
            return KeyStatus {
                banned: false,
                retry_after_secs: None,
            };
        };

        let now = Instant::now();
        if now < banned_until {
            KeyStatus {
                banned: true,
                retry_after_secs: Some((banned_until - now).as_secs().max(1)),
            }
        } else {
            KeyStatus {
                banned: false,
                retry_after_secs: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Algorithm;
    use super::KeyStrategy;
    use super::RateLimitConfig;
    use super::RateLimitState;

    fn state_with(algorithm: Algorithm, ban_threshold: u32) -> RateLimitState {
        RateLimitState {
            config: tokio::sync::RwLock::new(RateLimitConfig {
                algorithm,
                key_strategy: KeyStrategy::Ip,
                ban_threshold,
                ban_duration_ms: 200,
            }),
            keys: dashmap::DashMap::new(),
        }
    }

    #[tokio::test]
    async fn allows_requests_within_the_configured_limit() {
        let state = state_with(
            Algorithm::TokenBucket {
                capacity: 2,
                refill_per_sec: 0.001,
            },
            10,
        );

        assert_eq!(state.check("a").await, super::Decision::Allowed);
        assert_eq!(state.check("a").await, super::Decision::Allowed);
    }

    #[tokio::test]
    async fn limits_requests_beyond_the_configured_capacity() {
        let state = state_with(
            Algorithm::TokenBucket {
                capacity: 1,
                refill_per_sec: 0.001,
            },
            10,
        );

        assert_eq!(state.check("a").await, super::Decision::Allowed);
        assert!(matches!(
            state.check("a").await,
            super::Decision::Limited { .. }
        ));
    }

    #[tokio::test]
    async fn bans_a_key_after_reaching_the_violation_threshold() {
        let state = state_with(
            Algorithm::TokenBucket {
                capacity: 1,
                refill_per_sec: 0.001,
            },
            2,
        );

        assert_eq!(state.check("a").await, super::Decision::Allowed);
        assert!(matches!(
            state.check("a").await,
            super::Decision::Limited { .. }
        ));
        assert!(matches!(
            state.check("a").await,
            super::Decision::Banned { .. }
        ));
    }

    #[tokio::test]
    async fn a_ban_expires_after_its_configured_duration() {
        let state = state_with(
            Algorithm::TokenBucket {
                capacity: 1,
                refill_per_sec: 1000.0,
            },
            1,
        );

        assert_eq!(state.check("a").await, super::Decision::Allowed);
        assert!(matches!(
            state.check("a").await,
            super::Decision::Banned { .. }
        ));

        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

        assert_eq!(state.check("a").await, super::Decision::Allowed);
    }

    #[tokio::test]
    async fn different_keys_are_tracked_independently() {
        let state = state_with(
            Algorithm::TokenBucket {
                capacity: 1,
                refill_per_sec: 0.001,
            },
            10,
        );

        assert_eq!(state.check("a").await, super::Decision::Allowed);
        assert_eq!(state.check("b").await, super::Decision::Allowed);
    }

    #[tokio::test]
    async fn reset_clears_every_key() {
        let state = state_with(
            Algorithm::TokenBucket {
                capacity: 1,
                refill_per_sec: 0.001,
            },
            10,
        );

        assert_eq!(state.check("a").await, super::Decision::Allowed);
        assert!(matches!(
            state.check("a").await,
            super::Decision::Limited { .. }
        ));

        state.reset();

        assert_eq!(state.check("a").await, super::Decision::Allowed);
    }

    #[tokio::test]
    async fn configure_replaces_the_policy_and_clears_state() {
        let state = state_with(
            Algorithm::TokenBucket {
                capacity: 1,
                refill_per_sec: 0.001,
            },
            10,
        );

        assert_eq!(state.check("a").await, super::Decision::Allowed);

        state
            .configure(RateLimitConfig {
                algorithm: Algorithm::TokenBucket {
                    capacity: 5,
                    refill_per_sec: 0.001,
                },
                key_strategy: KeyStrategy::Ip,
                ban_threshold: 10,
                ban_duration_ms: 200,
            })
            .await;

        assert_eq!(state.check("a").await, super::Decision::Allowed);
    }

    #[test]
    fn status_reports_unbanned_for_an_unknown_key() {
        let state = RateLimitState::default();

        let status = state.status("unknown");

        assert!(!status.banned);
        assert!(status.retry_after_secs.is_none());
    }
}
