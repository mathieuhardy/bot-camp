//! Shared application state.

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::challenge::ChallengeState;
use crate::dashboard::DashboardState;
use crate::honeypot::HoneypotState;
use crate::rate_limit::RateLimitState;

/// Content served by `GET /robots.txt` until it's overridden via
/// `PUT /robots.txt`.
const DEFAULT_ROBOTS_TXT: &str = "User-agent: *\nAllow: /\n";

/// Shared, mutable state threaded through routes via axum's `State`
/// extractor.
#[derive(Clone)]
pub(crate) struct AppState {
    /// The current contents served by `GET /robots.txt`.
    pub(crate) robots_txt: Arc<RwLock<String>>,

    /// Rate limiting configuration and per-key counters.
    pub(crate) rate_limit: Arc<RateLimitState>,

    /// Honeypot configuration and every caught key's ban.
    pub(crate) honeypot: Arc<HoneypotState>,

    /// JS challenge configuration — no per-key state, since passing the
    /// gate depends only on the request's cookie.
    pub(crate) challenge: Arc<ChallengeState>,

    /// Live event feed consumed by the dashboard.
    pub(crate) dashboard: Arc<DashboardState>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            robots_txt: Arc::new(RwLock::new(DEFAULT_ROBOTS_TXT.to_string())),
            rate_limit: Arc::new(RateLimitState::default()),
            honeypot: Arc::new(HoneypotState::default()),
            challenge: Arc::new(ChallengeState::default()),
            dashboard: Arc::new(DashboardState::default()),
        }
    }
}
