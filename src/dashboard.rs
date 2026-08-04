//! Live event feed and point-in-time snapshot for the dashboard: a
//! broadcast channel the rate limiter and honeypot publish into, and the
//! data shapes sent to a newly connected client (once, as a snapshot) and
//! on every subsequent decision (as an event).

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Serialize;
use tokio::sync::broadcast;

use crate::challenge::ChallengeConfig;
use crate::honeypot::HoneypotConfig;
use crate::rate_limit::RateLimitConfig;

/// Number of in-flight events a slow WebSocket client can lag behind by
/// before the oldest ones are dropped for it.
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Which mechanism produced a [`DashboardEvent`].
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventSource {
    RateLimit,
    Honeypot,
}

/// One decision made by the rate limiter or the honeypot, broadcast to
/// every connected dashboard client.
#[derive(Clone, Serialize)]
pub(crate) struct DashboardEvent {
    /// Milliseconds since the Unix epoch, when the decision was made.
    pub(crate) timestamp_ms: u64,

    /// Which mechanism produced this event.
    pub(crate) source: EventSource,

    /// The rate limiting/honeypot key the decision was made for.
    pub(crate) key: String,

    /// The decision itself: `"allowed"`, `"limited"`, `"banned"`,
    /// `"blocked"`, or `"allow_listed"` from the rate limiter; `"blocked"`
    /// or `"trapped"` from the honeypot.
    pub(crate) decision: &'static str,

    /// Seconds remaining before the limit/ban clears, if the decision
    /// carries one.
    pub(crate) retry_after_secs: Option<u64>,
}

/// A rate limiter key's current state, as listed in a [`Snapshot`].
#[derive(Serialize)]
pub(crate) struct RateLimitKeyEntry {
    /// The key itself (an IP, a `User-Agent`, or both, depending on the
    /// configured strategy).
    pub(crate) key: String,

    /// Whether the key is currently banned.
    pub(crate) banned: bool,

    /// Seconds remaining on the ban, if any.
    pub(crate) retry_after_secs: Option<u64>,

    /// Consecutive violations recorded since the last time the key was
    /// allowed.
    pub(crate) consecutive_violations: u32,
}

/// A honeypot key's current state, as listed in a [`Snapshot`]. Every
/// entry is necessarily banned — the honeypot only ever remembers a key
/// once it has sprung the trap.
#[derive(Serialize)]
pub(crate) struct HoneypotKeyEntry {
    /// The key itself.
    pub(crate) key: String,

    /// Always `true` — kept for a shape consistent with
    /// [`RateLimitKeyEntry`].
    pub(crate) banned: bool,

    /// Seconds remaining on the ban.
    pub(crate) retry_after_secs: Option<u64>,
}

/// The rate limiter's configuration and every key it is currently
/// tracking.
#[derive(Serialize)]
pub(crate) struct RateLimitSnapshot {
    /// The current rate limiting policy.
    pub(crate) config: RateLimitConfig,

    /// Every key currently tracked.
    pub(crate) keys: Vec<RateLimitKeyEntry>,
}

/// The honeypot's configuration and every key it currently has banned.
#[derive(Serialize)]
pub(crate) struct HoneypotSnapshot {
    /// The current honeypot policy.
    pub(crate) config: HoneypotConfig,

    /// Every key currently banned.
    pub(crate) keys: Vec<HoneypotKeyEntry>,
}

/// A point-in-time view of the whole dashboard: sent once to a client
/// that just connected, so it can render the current state before the
/// live event feed carries it forward.
#[derive(Serialize)]
pub(crate) struct Snapshot {
    /// The rate limiter's configuration and tracked keys.
    pub(crate) rate_limit: RateLimitSnapshot,

    /// The honeypot's configuration and banned keys.
    pub(crate) honeypot: HoneypotSnapshot,

    /// The JS challenge's configuration — it has no per-key state to
    /// report.
    pub(crate) challenge: ChallengeConfig,
}

/// A message sent over the dashboard WebSocket: the initial snapshot,
/// then a stream of events.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum DashboardMessage {
    Snapshot(Snapshot),
    Event(DashboardEvent),
}

/// Shared dashboard state: a broadcast channel every connected client
/// subscribes to, fed by the rate limiter and honeypot middlewares.
pub(crate) struct DashboardState {
    events: broadcast::Sender<DashboardEvent>,
}

impl Default for DashboardState {
    fn default() -> Self {
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);

        DashboardState { events }
    }
}

impl DashboardState {
    /// Broadcasts `event` to every currently connected client. Silently
    /// does nothing if none are connected — the dashboard is an optional
    /// observability feature, not a required delivery channel.
    pub(crate) fn publish(&self, event: DashboardEvent) {
        let _ = self.events.send(event);
    }

    /// Subscribes to the live event feed, starting from this call
    /// onward.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<DashboardEvent> {
        self.events.subscribe()
    }
}

/// The current time, as milliseconds since the Unix epoch — used to
/// timestamp [`DashboardEvent`]s.
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::DashboardEvent;
    use super::DashboardState;
    use super::EventSource;

    fn event(key: &str) -> DashboardEvent {
        DashboardEvent {
            timestamp_ms: 0,
            source: EventSource::RateLimit,
            key: key.to_string(),
            decision: "allowed",
            retry_after_secs: None,
        }
    }

    #[tokio::test]
    async fn a_subscriber_receives_a_published_event() {
        let state = DashboardState::default();
        let mut receiver = state.subscribe();

        state.publish(event("a"));

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.key, "a");
    }

    #[test]
    fn publishing_without_a_subscriber_does_not_panic() {
        let state = DashboardState::default();

        state.publish(event("a"));
    }
}
