//! Redirect endpoints: single-hop, chained, looping, and header- or
//! meta-tag-based (`Refresh`) redirects.

use axum::extract::Path;
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header::LOCATION;
use axum::response::Html;
use serde::Deserialize;

use crate::error::Error;
use crate::error::Result;
use crate::templates::PageContext;
use crate::templates::render_page;

/// HTTP status codes that represent an HTTP redirect.
const REDIRECT_CODES: [u16; 6] = [300, 301, 302, 303, 307, 308];

/// Status code used for each hop of [`redirect_chain`] and [`redirect_loop`].
const HOP_STATUS: StatusCode = StatusCode::FOUND;

/// Query parameters accepted by [`redirect`].
#[derive(Deserialize)]
pub(crate) struct RedirectParams {
    /// The URL to redirect to, absolute or relative.
    to: String,
}

/// Query parameters accepted by [`redirect_chain`].
#[derive(Deserialize)]
pub(crate) struct ChainParams {
    /// Number of redirect hops remaining before landing on `to`.
    n: u32,

    /// The URL to land on once the chain completes.
    to: String,
}

/// Query parameters accepted by [`redirect_loop`].
#[derive(Deserialize)]
pub(crate) struct LoopParams {
    /// Total number of positions in the loop.
    steps: u32,

    /// Current position in the loop.
    #[serde(default)]
    step: u32,
}

/// Query parameters accepted by [`redirect_refresh`] and
/// [`redirect_meta_refresh`].
#[derive(Deserialize)]
pub(crate) struct RefreshParams {
    /// Delay, in seconds, announced in the refresh.
    delay: u64,

    /// The URL to redirect to.
    to: String,
}

/// Redirects to `to` with the given HTTP status `code`.
///
/// # Returns
/// `code` with a `Location: to` header, or an `Error` if `code` isn't a
/// redirect status (`300`-`303`, `307`, `308`), or `to` isn't a valid
/// header value.
pub async fn redirect(
    Path(code): Path<u16>,
    Query(params): Query<RedirectParams>,
) -> Result<(StatusCode, HeaderMap)> {
    let status = redirect_status(code)?;
    let headers = location_header(&params.to)?;

    Ok((status, headers))
}

/// Redirects through `n` intermediate hops before landing on `to`.
///
/// # Returns
/// A `302 Found` pointing either at the next hop (decrementing `n`), or,
/// once `n` reaches `0`, directly at `to`. An `Error` if `to` isn't a
/// valid header value.
pub async fn redirect_chain(Query(params): Query<ChainParams>) -> Result<(StatusCode, HeaderMap)> {
    let target = if params.n == 0 {
        params.to
    } else {
        format!("/redirect/chain?n={}&to={}", params.n - 1, params.to)
    };
    let headers = location_header(&target)?;

    Ok((HOP_STATUS, headers))
}

/// Redirects forever, cycling through `steps` positions.
///
/// # Returns
/// A `302 Found` pointing at the next position in the loop. An `Error`
/// if `steps` is `0`.
pub async fn redirect_loop(Query(params): Query<LoopParams>) -> Result<(StatusCode, HeaderMap)> {
    if params.steps == 0 {
        return Err(Error::InvalidRedirectSteps(params.steps));
    }

    let next = (params.step + 1) % params.steps;
    let target = format!("/redirect/loop?steps={}&step={next}", params.steps);
    let headers = location_header(&target)?;

    Ok((HOP_STATUS, headers))
}

/// Redirects to `to` via a `Refresh` response header instead of a real
/// HTTP redirect status, the way old-school "you will be redirected in N
/// seconds" pages do.
///
/// # Returns
/// `200 OK` with a `Refresh: delay; url=to` header, or an `Error` if `to`
/// isn't a valid header value.
pub async fn redirect_refresh(Query(params): Query<RefreshParams>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("refresh"),
        HeaderValue::from_str(&refresh_content(&params))?,
    );

    Ok(headers)
}

/// Redirects to `to` via an HTML `<meta http-equiv="refresh">` tag
/// instead of a real HTTP redirect status or the `Refresh` header, the
/// way old-school "you will be redirected in N seconds" pages do.
///
/// # Returns
/// `200 OK` with an HTML page whose `<head>` holds
/// `<meta http-equiv="refresh" content="delay; url=to">`.
pub async fn redirect_meta_refresh(Query(params): Query<RefreshParams>) -> Html<String> {
    let context = PageContext {
        titles: vec!["Meta refresh".to_string()],
        refresh: Some(refresh_content(&params)),
        body: "Redirecting…".to_string(),
        ..Default::default()
    };

    Html(render_page(context))
}

/// Builds the `delay; url=to` value shared by the header-based and
/// meta-tag-based refresh redirects.
fn refresh_content(params: &RefreshParams) -> String {
    format!("{}; url={}", params.delay, params.to)
}

/// Returns `code` as a `StatusCode` if it represents an HTTP redirect, or
/// an `Error` otherwise.
fn redirect_status(code: u16) -> Result<StatusCode> {
    if !REDIRECT_CODES.contains(&code) {
        return Err(Error::InvalidRedirectCode(code));
    }

    Ok(StatusCode::from_u16(code).unwrap())
}

/// Builds a `Location` header pointing at `to`.
pub(crate) fn location_header(to: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(LOCATION, HeaderValue::from_str(to)?);

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use axum::extract::Path;
    use axum::extract::Query;
    use axum::http::StatusCode;
    use axum::http::header::LOCATION;

    use super::ChainParams;
    use super::LoopParams;
    use super::RedirectParams;
    use super::RefreshParams;
    use super::redirect;
    use super::redirect_chain;
    use super::redirect_loop;
    use super::redirect_meta_refresh;
    use super::redirect_refresh;

    #[tokio::test]
    async fn redirect_returns_the_requested_code_and_location() {
        let params = RedirectParams {
            to: "/status/200".to_string(),
        };

        let (status, headers) = redirect(Path(301), Query(params)).await.unwrap();

        assert_eq!(status, StatusCode::MOVED_PERMANENTLY);
        assert_eq!(headers.get(LOCATION).unwrap(), "/status/200");
    }

    #[tokio::test]
    async fn redirect_rejects_a_non_redirect_code() {
        let params = RedirectParams {
            to: "/status/200".to_string(),
        };

        let result = redirect(Path(200), Query(params)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn chain_points_at_the_next_hop_while_n_is_positive() {
        let params = ChainParams {
            n: 2,
            to: "/status/200".to_string(),
        };

        let (status, headers) = redirect_chain(Query(params)).await.unwrap();

        assert_eq!(status, StatusCode::FOUND);
        assert_eq!(
            headers.get(LOCATION).unwrap(),
            "/redirect/chain?n=1&to=/status/200"
        );
    }

    #[tokio::test]
    async fn chain_lands_on_to_once_n_reaches_zero() {
        let params = ChainParams {
            n: 0,
            to: "/status/200".to_string(),
        };

        let (_, headers) = redirect_chain(Query(params)).await.unwrap();

        assert_eq!(headers.get(LOCATION).unwrap(), "/status/200");
    }

    #[tokio::test]
    async fn redirect_loop_wraps_around_to_the_first_position() {
        let params = LoopParams { steps: 3, step: 2 };

        let (_, headers) = redirect_loop(Query(params)).await.unwrap();

        assert_eq!(
            headers.get(LOCATION).unwrap(),
            "/redirect/loop?steps=3&step=0"
        );
    }

    #[tokio::test]
    async fn redirect_loop_rejects_zero_steps() {
        let params = LoopParams { steps: 0, step: 0 };

        let result = redirect_loop(Query(params)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn refresh_builds_the_expected_header_value() {
        let params = RefreshParams {
            delay: 5,
            to: "/status/200".to_string(),
        };

        let headers = redirect_refresh(Query(params)).await.unwrap();

        assert_eq!(headers.get("refresh").unwrap(), "5; url=/status/200");
    }

    #[tokio::test]
    async fn meta_refresh_embeds_the_expected_tag() {
        let params = RefreshParams {
            delay: 5,
            to: "/status/200".to_string(),
        };

        let html = redirect_meta_refresh(Query(params)).await.0;

        assert!(html.contains(r#"<meta http-equiv="refresh" content="5; url=/status/200">"#));
    }
}
