//! Generic, ultra-configurable response endpoint: describe a status
//! code, arbitrary headers, a delay, and a body — either raw text or a
//! full [`PageContext`] — in one JSON request, composing what the
//! single-purpose endpoints elsewhere in this crate each do separately.

use std::time::Duration;

use axum::Json;
use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::response::Response;
use serde::Deserialize;
use tokio::time::sleep;

use crate::error::Error;
use crate::error::Result;
use crate::templates::PageContext;
use crate::templates::render_page;

/// One response header to set, by name and value — a list (rather than
/// a JSON object) so the same name can appear more than once.
#[derive(Deserialize)]
pub(crate) struct HeaderEntry {
    /// The header's name.
    pub(crate) name: String,

    /// The header's value.
    pub(crate) value: String,
}

/// Request body accepted by [`response`].
#[derive(Deserialize)]
pub(crate) struct GenericResponseRequest {
    /// The status code to respond with. Defaults to `200`.
    #[serde(default = "default_status")]
    status: u16,

    /// Response headers to set, in order — a name repeated across
    /// several entries produces that many header lines.
    #[serde(default)]
    headers: Vec<HeaderEntry>,

    /// Milliseconds to wait before responding. Defaults to `0`.
    #[serde(default)]
    delay_ms: u64,

    /// Raw response body text, sent verbatim. Mutually exclusive with
    /// `page`.
    #[serde(default)]
    body: Option<String>,

    /// A full page description, rendered through the same shared HTML
    /// skeleton every other content-based endpoint uses. Mutually
    /// exclusive with `body`.
    #[serde(default)]
    page: Option<PageContext>,
}

fn default_status() -> u16 {
    200
}

/// Composes a response from `request`: waits `delay_ms` if any, then
/// responds with `status`, `headers`, and a body built from `body` or
/// `page` (whichever was given).
///
/// # Returns
/// The composed response, or an `Error` if `status`/a header is invalid,
/// or if both `body` and `page` were given.
pub async fn response(Json(request): Json<GenericResponseRequest>) -> Result<Response> {
    if request.body.is_some() && request.page.is_some() {
        return Err(Error::ConflictingBody);
    }

    if request.delay_ms > 0 {
        sleep(Duration::from_millis(request.delay_ms)).await;
    }

    let status = StatusCode::from_u16(request.status)?;

    let (body, is_html) = match (request.body, request.page) {
        (Some(raw), None) => (raw, false),
        (None, Some(page)) => (render_page(page), true),
        (None, None) => (String::new(), false),
        (Some(_), Some(_)) => unreachable!("checked above"),
    };

    let overrides_content_type = request
        .headers
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case("content-type"));

    let mut response_headers = HeaderMap::new();
    if is_html && !overrides_content_type {
        response_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
    }

    for entry in request.headers {
        let name = HeaderName::from_bytes(entry.name.as_bytes())?;
        let value = HeaderValue::from_str(&entry.value)?;
        response_headers.append(name, value);
    }

    Ok((status, response_headers, body).into_response())
}

#[cfg(test)]
mod tests {
    use axum::Json;
    use axum::http::StatusCode;

    use super::GenericResponseRequest;
    use super::HeaderEntry;
    use super::response;
    use crate::templates::PageContext;

    fn request(
        status: u16,
        headers: Vec<HeaderEntry>,
        body: Option<String>,
        page: Option<PageContext>,
    ) -> GenericResponseRequest {
        GenericResponseRequest {
            status,
            headers,
            delay_ms: 0,
            body,
            page,
        }
    }

    #[tokio::test]
    async fn defaults_to_200_with_an_empty_body() {
        let result = response(Json(request(200, Vec::new(), None, None)))
            .await
            .unwrap();

        assert_eq!(result.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_an_out_of_range_status_code() {
        let result = response(Json(request(0, Vec::new(), None, None))).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_body_and_page_given_together() {
        let result = response(Json(request(
            200,
            Vec::new(),
            Some("raw".to_string()),
            Some(PageContext::default()),
        )))
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn repeats_a_header_entry_with_the_same_name() {
        let headers = vec![
            HeaderEntry {
                name: "x-foo".to_string(),
                value: "bar".to_string(),
            },
            HeaderEntry {
                name: "x-foo".to_string(),
                value: "baz".to_string(),
            },
        ];

        let result = response(Json(request(200, headers, None, None)))
            .await
            .unwrap();

        let values: Vec<_> = result.headers().get_all("x-foo").iter().collect();
        assert_eq!(values, vec!["bar", "baz"]);
    }

    #[tokio::test]
    async fn defaults_content_type_to_html_when_page_is_given() {
        let result = response(Json(request(
            200,
            Vec::new(),
            None,
            Some(PageContext::default()),
        )))
        .await
        .unwrap();

        assert_eq!(
            result.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn an_explicit_content_type_overrides_the_html_default_without_duplicating() {
        let headers = vec![HeaderEntry {
            name: "content-type".to_string(),
            value: "application/xhtml+xml".to_string(),
        }];

        let result = response(Json(request(
            200,
            headers,
            None,
            Some(PageContext::default()),
        )))
        .await
        .unwrap();

        let values: Vec<_> = result.headers().get_all("content-type").iter().collect();
        assert_eq!(values, vec!["application/xhtml+xml"]);
    }
}
