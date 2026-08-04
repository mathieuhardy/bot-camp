//! The dashboard: a Svelte single-page app embedded into the binary at
//! compile time from `frontend/dist` (see `frontend/README.md`), plus the
//! JSON/WebSocket endpoints it consumes.

use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::response::Response;
use rust_embed::RustEmbed;
use tokio::sync::broadcast;

use crate::dashboard::DashboardMessage;
use crate::dashboard::HoneypotSnapshot;
use crate::dashboard::RateLimitSnapshot;
use crate::dashboard::Snapshot;
use crate::state::AppState;

/// Static assets for the dashboard's single-page app.
#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

/// Serves the dashboard's `index.html`.
///
/// # Returns
/// `200 OK` with the embedded page.
pub async fn index() -> Response {
    serve_asset("index.html")
}

/// Serves any other embedded asset (the JS/CSS bundles under `assets/`,
/// the favicon, etc.) by its path relative to `frontend/dist`.
///
/// # Returns
/// `200 OK` with the embedded file; `404 Not Found` if it doesn't exist.
pub async fn assets(Path(path): Path<String>) -> Response {
    serve_asset(&path)
}

/// Looks up `path` among the embedded dashboard assets.
fn serve_asset(path: &str) -> Response {
    let Some(file) = Assets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    (
        [(CONTENT_TYPE, file.metadata.mimetype().to_string())],
        file.data.into_owned(),
    )
        .into_response()
}

/// Returns a point-in-time view of the rate limiter, honeypot, and
/// challenge — the same shape sent once over [`ws`] when a client
/// connects.
///
/// # Returns
/// `200 OK` with the snapshot as JSON.
pub async fn snapshot(State(state): State<AppState>) -> Json<Snapshot> {
    Json(build_snapshot(&state).await)
}

/// Upgrades to a WebSocket connection: sends the current snapshot once,
/// then relays every subsequent event until the client disconnects.
///
/// # Returns
/// A WebSocket upgrade response.
pub async fn ws(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Drives one dashboard client's WebSocket connection for its whole
/// lifetime.
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let snapshot = build_snapshot(&state).await;
    if send(&mut socket, &DashboardMessage::Snapshot(snapshot))
        .await
        .is_err()
    {
        return;
    }

    let mut events = state.dashboard.subscribe();

    loop {
        tokio::select! {
            event = events.recv() => {
                match event {
                    Ok(event) => {
                        if send(&mut socket, &DashboardMessage::Event(event)).await.is_err() {
                            break;
                        }
                    }

                    // A slow client just missed some events; keep going.
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,

                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            // Not interested in anything the client sends, but polling is
            // what detects it closing the connection.
            incoming = socket.recv() => {
                if !matches!(incoming, Some(Ok(_))) {
                    break;
                }
            }
        }
    }
}

/// Serializes `message` and sends it as a WebSocket text frame.
async fn send(socket: &mut WebSocket, message: &DashboardMessage) -> Result<(), axum::Error> {
    let text = serde_json::to_string(message).expect("DashboardMessage always serializes");

    socket.send(Message::Text(text.into())).await
}

/// Assembles the current [`Snapshot`] from every mechanism's state.
async fn build_snapshot(state: &AppState) -> Snapshot {
    Snapshot {
        rate_limit: RateLimitSnapshot {
            config: state.rate_limit.config().await,
            keys: state.rate_limit.snapshot(),
        },

        honeypot: HoneypotSnapshot {
            config: state.honeypot.config().await,
            keys: state.honeypot.snapshot(),
        },

        challenge: state.challenge.config().await,
    }
}
