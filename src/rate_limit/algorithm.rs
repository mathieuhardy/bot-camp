//! Pluggable rate limiting algorithms: token bucket, fixed window,
//! sliding window, and minimal interval.

use std::time::Duration;
use std::time::Instant;

use serde::Deserialize;

/// A rate limiting algorithm and its parameters.
#[derive(Clone, Deserialize)]
#[serde(tag = "algorithm", rename_all = "snake_case")]
pub(crate) enum Algorithm {
    /// Allows bursts up to `capacity` requests, then refills at
    /// `refill_per_sec` tokens per second.
    TokenBucket { capacity: u32, refill_per_sec: f64 },

    /// Allows at most `limit` requests per `window_ms`-long fixed
    /// window. Simple, but tolerates a burst of up to `2 * limit`
    /// requests around a window boundary.
    FixedWindow { limit: u32, window_ms: u64 },

    /// Allows at most `limit` requests per `window_ms`, approximated as
    /// a weighted count across the current and previous fixed windows —
    /// smooths out the fixed-window boundary burst.
    SlidingWindow { limit: u32, window_ms: u64 },

    /// Rejects a request if it arrives less than `min_interval_ms` after
    /// the key's previous request (accepted or not) — a strict pacing
    /// check, distinct from a rate over a window: it catches a crawler
    /// that ignores `Crawl-delay` even if it stays under a broader
    /// quota.
    MinInterval { min_interval_ms: u64 },
}

/// Per-key bookkeeping for whichever [`Algorithm`] is configured.
pub(crate) enum AlgorithmState {
    TokenBucket {
        tokens: f64,
        last_refill: Instant,
    },

    FixedWindow {
        window_start: Instant,
        count: u32,
    },

    SlidingWindow {
        window_start: Instant,
        current_count: u32,
        previous_count: u32,
    },

    MinInterval {
        last_seen: Option<Instant>,
    },
}

/// Outcome of one [`AlgorithmState::check`] call.
pub(crate) enum AlgorithmDecision {
    Allowed,
    Limited { retry_after_secs: u64 },
}

impl AlgorithmState {
    /// Creates fresh bookkeeping for a brand-new key, matching
    /// `algorithm`.
    pub(crate) fn new(algorithm: &Algorithm, now: Instant) -> Self {
        match algorithm {
            Algorithm::TokenBucket { capacity, .. } => AlgorithmState::TokenBucket {
                tokens: f64::from(*capacity),
                last_refill: now,
            },

            Algorithm::FixedWindow { .. } => AlgorithmState::FixedWindow {
                window_start: now,
                count: 0,
            },

            Algorithm::SlidingWindow { .. } => AlgorithmState::SlidingWindow {
                window_start: now,
                current_count: 0,
                previous_count: 0,
            },

            Algorithm::MinInterval { .. } => AlgorithmState::MinInterval { last_seen: None },
        }
    }

    /// Records one request against `algorithm` at `now`, returning
    /// whether it's allowed.
    ///
    /// # Panics
    /// If `algorithm` doesn't match the variant this state was created
    /// from — can't happen in practice, since [`super::RateLimitState`]
    /// always creates state from the same config it checks against.
    pub(crate) fn check(&mut self, algorithm: &Algorithm, now: Instant) -> AlgorithmDecision {
        match (self, algorithm) {
            (
                AlgorithmState::TokenBucket {
                    tokens,
                    last_refill,
                },
                Algorithm::TokenBucket {
                    capacity,
                    refill_per_sec,
                },
            ) => check_token_bucket(tokens, last_refill, *capacity, *refill_per_sec, now),

            (
                AlgorithmState::FixedWindow {
                    window_start,
                    count,
                },
                Algorithm::FixedWindow { limit, window_ms },
            ) => check_fixed_window(window_start, count, *limit, *window_ms, now),

            (
                AlgorithmState::SlidingWindow {
                    window_start,
                    current_count,
                    previous_count,
                },
                Algorithm::SlidingWindow { limit, window_ms },
            ) => check_sliding_window(
                window_start,
                current_count,
                previous_count,
                *limit,
                *window_ms,
                now,
            ),

            (
                AlgorithmState::MinInterval { last_seen },
                Algorithm::MinInterval { min_interval_ms },
            ) => check_min_interval(last_seen, *min_interval_ms, now),

            _ => unreachable!("algorithm state always matches the algorithm it was created from"),
        }
    }
}

/// Refills `tokens` for the elapsed time since `last_refill`, then
/// consumes one if available.
fn check_token_bucket(
    tokens: &mut f64,
    last_refill: &mut Instant,
    capacity: u32,
    refill_per_sec: f64,
    now: Instant,
) -> AlgorithmDecision {
    let elapsed = now.saturating_duration_since(*last_refill).as_secs_f64();
    *tokens = (*tokens + elapsed * refill_per_sec).min(f64::from(capacity));
    *last_refill = now;

    if *tokens >= 1.0 {
        *tokens -= 1.0;
        return AlgorithmDecision::Allowed;
    }

    let missing = 1.0 - *tokens;
    let retry_after_secs = (missing / refill_per_sec).ceil().max(1.0) as u64;

    AlgorithmDecision::Limited { retry_after_secs }
}

/// Resets `count` once `window_ms` has elapsed since `window_start`,
/// then counts one request against the (possibly fresh) window.
fn check_fixed_window(
    window_start: &mut Instant,
    count: &mut u32,
    limit: u32,
    window_ms: u64,
    now: Instant,
) -> AlgorithmDecision {
    let window = Duration::from_millis(window_ms);

    if now.saturating_duration_since(*window_start) >= window {
        *window_start = now;
        *count = 0;
    }

    if *count < limit {
        *count += 1;
        return AlgorithmDecision::Allowed;
    }

    let retry_after = window - now.saturating_duration_since(*window_start);

    AlgorithmDecision::Limited {
        retry_after_secs: retry_after.as_secs().max(1),
    }
}

/// Rolls `window_start`/`current_count`/`previous_count` forward as
/// needed, then weighs the previous window's count by how much of it
/// still "overlaps" the current moment, per the standard sliding-window
/// counter approximation (no per-request timestamp log to keep memory
/// bounded).
fn check_sliding_window(
    window_start: &mut Instant,
    current_count: &mut u32,
    previous_count: &mut u32,
    limit: u32,
    window_ms: u64,
    now: Instant,
) -> AlgorithmDecision {
    let window = Duration::from_millis(window_ms);
    let elapsed = now.saturating_duration_since(*window_start);

    if elapsed >= window * 2 {
        *window_start = now;
        *current_count = 0;
        *previous_count = 0;
    } else if elapsed >= window {
        *window_start += window;
        *previous_count = *current_count;
        *current_count = 0;
    }

    let elapsed_in_current = now.saturating_duration_since(*window_start).as_secs_f64();
    let weight = 1.0 - (elapsed_in_current / window.as_secs_f64()).min(1.0);
    let effective_count = f64::from(*previous_count) * weight + f64::from(*current_count);

    if effective_count < f64::from(limit) {
        *current_count += 1;
        return AlgorithmDecision::Allowed;
    }

    // The weighted count makes an exact retry-after nontrivial; one
    // second is a reasonable, conservative approximation.
    AlgorithmDecision::Limited {
        retry_after_secs: 1,
    }
}

/// Rejects a request that arrives less than `min_interval_ms` after
/// `last_seen`, then always records `now` as the new `last_seen` — even
/// on rejection, so a key that keeps hammering must wait the full
/// interval from its most recent attempt, not just its last accepted
/// one.
fn check_min_interval(
    last_seen: &mut Option<Instant>,
    min_interval_ms: u64,
    now: Instant,
) -> AlgorithmDecision {
    let min_interval = Duration::from_millis(min_interval_ms);

    let decision = match *last_seen {
        Some(previous) if now.saturating_duration_since(previous) < min_interval => {
            let retry_after = min_interval - now.saturating_duration_since(previous);

            AlgorithmDecision::Limited {
                retry_after_secs: retry_after.as_secs().max(1),
            }
        }

        _ => AlgorithmDecision::Allowed,
    };

    *last_seen = Some(now);

    decision
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::time::Instant;

    use super::Algorithm;
    use super::AlgorithmDecision;
    use super::AlgorithmState;

    #[test]
    fn token_bucket_allows_bursts_up_to_capacity() {
        let algorithm = Algorithm::TokenBucket {
            capacity: 3,
            refill_per_sec: 1.0,
        };
        let now = Instant::now();
        let mut state = AlgorithmState::new(&algorithm, now);

        for _ in 0..3 {
            assert!(matches!(
                state.check(&algorithm, now),
                AlgorithmDecision::Allowed
            ));
        }
        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Limited { .. }
        ));
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let algorithm = Algorithm::TokenBucket {
            capacity: 1,
            refill_per_sec: 2.0,
        };
        let now = Instant::now();
        let mut state = AlgorithmState::new(&algorithm, now);

        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));
        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Limited { .. }
        ));

        let later = now + Duration::from_millis(600);
        assert!(matches!(
            state.check(&algorithm, later),
            AlgorithmDecision::Allowed
        ));
    }

    #[test]
    fn fixed_window_resets_after_the_window_elapses() {
        let algorithm = Algorithm::FixedWindow {
            limit: 2,
            window_ms: 1000,
        };
        let now = Instant::now();
        let mut state = AlgorithmState::new(&algorithm, now);

        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));
        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));
        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Limited { .. }
        ));

        let next_window = now + Duration::from_millis(1001);
        assert!(matches!(
            state.check(&algorithm, next_window),
            AlgorithmDecision::Allowed
        ));
    }

    #[test]
    fn sliding_window_weighs_the_previous_window_down_over_time() {
        let algorithm = Algorithm::SlidingWindow {
            limit: 2,
            window_ms: 1000,
        };
        let now = Instant::now();
        let mut state = AlgorithmState::new(&algorithm, now);

        // Fill the first window
        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));
        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));

        // Early in the next window, the previous window's weight is
        // still high enough that a second request denies (the first
        // still slips through, since the weighted previous count alone
        // is just under the limit)
        let early_next = now + Duration::from_millis(1050);
        assert!(matches!(
            state.check(&algorithm, early_next),
            AlgorithmDecision::Allowed
        ));
        assert!(matches!(
            state.check(&algorithm, early_next),
            AlgorithmDecision::Limited { .. }
        ));

        // Late in the next window, the previous window's weight has
        // decayed enough to allow again
        let late_next = now + Duration::from_millis(1950);
        assert!(matches!(
            state.check(&algorithm, late_next),
            AlgorithmDecision::Allowed
        ));
    }

    #[test]
    fn min_interval_allows_the_first_request() {
        let algorithm = Algorithm::MinInterval {
            min_interval_ms: 1000,
        };
        let now = Instant::now();
        let mut state = AlgorithmState::new(&algorithm, now);

        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));
    }

    #[test]
    fn min_interval_limits_a_request_that_arrives_too_soon() {
        let algorithm = Algorithm::MinInterval {
            min_interval_ms: 1000,
        };
        let now = Instant::now();
        let mut state = AlgorithmState::new(&algorithm, now);

        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));

        let too_soon = now + Duration::from_millis(500);
        assert!(matches!(
            state.check(&algorithm, too_soon),
            AlgorithmDecision::Limited { .. }
        ));
    }

    #[test]
    fn min_interval_allows_again_once_the_interval_has_elapsed() {
        let algorithm = Algorithm::MinInterval {
            min_interval_ms: 1000,
        };
        let now = Instant::now();
        let mut state = AlgorithmState::new(&algorithm, now);

        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));

        let later = now + Duration::from_millis(1000);
        assert!(matches!(
            state.check(&algorithm, later),
            AlgorithmDecision::Allowed
        ));
    }

    #[test]
    fn min_interval_keeps_rejecting_a_key_that_keeps_hammering() {
        let algorithm = Algorithm::MinInterval {
            min_interval_ms: 1000,
        };
        let now = Instant::now();
        let mut state = AlgorithmState::new(&algorithm, now);

        assert!(matches!(
            state.check(&algorithm, now),
            AlgorithmDecision::Allowed
        ));

        // Each rejected attempt still pushes `last_seen` forward, so
        // arriving 500ms after the *previous rejection* is still too
        // soon relative to a fresh 1000ms window.
        let first_retry = now + Duration::from_millis(500);
        assert!(matches!(
            state.check(&algorithm, first_retry),
            AlgorithmDecision::Limited { .. }
        ));

        let second_retry = first_retry + Duration::from_millis(500);
        assert!(matches!(
            state.check(&algorithm, second_retry),
            AlgorithmDecision::Limited { .. }
        ));
    }
}
