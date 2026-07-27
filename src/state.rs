//! Shared application state.

use std::sync::Arc;

use tokio::sync::RwLock;

/// Content served by `GET /robots.txt` until it's overridden via
/// `PUT /robots.txt`.
const DEFAULT_ROBOTS_TXT: &str = "User-agent: *\nAllow: /\n";

/// Shared, mutable state threaded through routes via axum's `State`
/// extractor.
#[derive(Clone)]
pub(crate) struct AppState {
    /// The current contents served by `GET /robots.txt`.
    pub(crate) robots_txt: Arc<RwLock<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            robots_txt: Arc::new(RwLock::new(DEFAULT_ROBOTS_TXT.to_string())),
        }
    }
}
