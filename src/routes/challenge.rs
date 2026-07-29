//! JS challenge playground, plus its config endpoint.
//!
//! Any path under `/challenge/` is gated by the [`enforce`] middleware: a
//! request without the validation cookie gets the challenge page instead
//! of [`probe`]'s response.

use axum::Json;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Response;

use crate::challenge::COOKIE_NAME;
use crate::challenge::COOKIE_VALUE;
use crate::challenge::ChallengeConfig;
use crate::challenge::is_solved;
use crate::state::AppState;
use crate::templates::PageContext;
use crate::templates::render_page;

/// Axum middleware gating every request under `/challenge/*`: a request
/// without the validation cookie gets [`challenge_page`] instead of
/// reaching [`probe`].
///
/// # Returns
/// [`probe`]'s response if the request already carries a valid cookie;
/// the challenge page otherwise.
pub(crate) async fn enforce(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if is_solved(request.headers()) {
        return next.run(request).await;
    }

    let config = state.challenge.config().await;

    Html(challenge_page(&config)).into_response()
}

/// Renders the "checking your browser" page: a deferred script that
/// waits `delay_ms` before setting the validation cookie and reloading.
fn challenge_page(config: &ChallengeConfig) -> String {
    let context = PageContext {
        body: "Checking your browser before accessing bot-camp...".to_string(),
        deferred_script: Some(format!(
            "setTimeout(function() {{ document.cookie = \"{COOKIE_NAME}={COOKIE_VALUE}; path=/; max-age={}\"; location.reload(); }}, {});",
            config.cookie_max_age_secs, config.delay_ms,
        )),
        ..Default::default()
    };

    render_page(context)
}

/// Playground page reached once a request clears [`enforce`] — any path
/// under `/challenge/` lands here.
///
/// # Returns
/// `200 OK` with a short acknowledgement.
pub async fn probe(Path(path): Path<String>) -> String {
    format!("ok: /challenge/{path}")
}

/// Replaces the current challenge configuration.
///
/// # Returns
/// `200 OK` once the new configuration is in effect.
pub async fn set_config(
    State(state): State<AppState>,
    Json(config): Json<ChallengeConfig>,
) -> StatusCode {
    state.challenge.configure(config).await;

    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::ChallengeConfig;
    use super::challenge_page;

    #[test]
    fn renders_the_expected_cookie_name_and_value() {
        let html = challenge_page(&ChallengeConfig {
            delay_ms: 1500,
            cookie_max_age_secs: 3600,
        });

        assert!(html.contains("botcamp_challenge=ok"));
    }

    #[test]
    fn honors_the_configured_delay_and_max_age() {
        let html = challenge_page(&ChallengeConfig {
            delay_ms: 2500,
            cookie_max_age_secs: 60,
        });

        assert!(html.contains("max-age=60"));
        assert!(html.contains("}, 2500);"));
    }
}
