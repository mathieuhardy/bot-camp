//! URL normalization endpoint.

use axum::extract::Query;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use serde::Deserialize;
use url::Url;

use crate::error::Result;
use crate::routes::redirect::location_header;

/// Query parameters accepted by [`normalize`].
#[derive(Deserialize)]
pub(crate) struct NormalizeParams {
    /// The URL to normalize.
    url: String,

    /// Collapse leading, trailing, and duplicated dots in the host
    /// (`example.com..` -> `example.com`).
    #[serde(default = "default_true")]
    remove_host_dots: bool,

    /// Strip a single trailing slash from the path, unless the path is
    /// just `/`.
    #[serde(default = "default_true")]
    remove_trailing_slash: bool,

    /// Sort query parameters alphabetically.
    #[serde(default = "default_true")]
    sort_query: bool,

    /// Drop the fragment (`#...`) entirely.
    #[serde(default = "default_true")]
    remove_fragment: bool,
}

/// Defaults every [`NormalizeParams`] flag to enabled.
fn default_true() -> bool {
    true
}

/// Normalizes `url` according to the requested flags, and redirects to
/// the result — the way a real server canonicalizes a URL, so a crawler
/// exercises its normal redirect-following path instead of having to
/// parse a bespoke response format.
///
/// Scheme and host are always lowercased, the default port for the
/// scheme is always dropped, and dot-segments (`.`, `..`) in the path are
/// always resolved — this is handled by URL parsing itself and can't be
/// turned off. Path segments and query keys/values keep their case
/// unchanged, since case sensitivity there is meaningful.
///
/// # Returns
/// `301 Moved Permanently` with a `Location` header pointing at the
/// normalized URL, if it differs from `url`; `200 OK` otherwise. An
/// `Error` if `url` doesn't parse.
pub async fn normalize(Query(params): Query<NormalizeParams>) -> Result<(StatusCode, HeaderMap)> {
    let mut url = Url::parse(&params.url)?;

    if params.remove_host_dots {
        url = remove_host_dots(url)?;
    }

    if params.remove_trailing_slash {
        remove_trailing_slash(&mut url);
    }

    if params.sort_query {
        sort_query(&mut url);
    }

    if params.remove_fragment {
        url.set_fragment(None);
    }

    let normalized = url.to_string();

    if normalized == params.url {
        return Ok((StatusCode::OK, HeaderMap::new()));
    }

    Ok((StatusCode::MOVED_PERMANENTLY, location_header(&normalized)?))
}

/// Collapses leading, trailing, and duplicated dots in `url`'s host.
fn remove_host_dots(mut url: Url) -> Result<Url> {
    let Some(host) = url.host_str() else {
        return Ok(url);
    };

    let cleaned = host
        .split('.')
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>()
        .join(".");

    url.set_host(Some(&cleaned))?;

    Ok(url)
}

/// Strips a single trailing slash from `url`'s path, unless the path is
/// just `/`.
fn remove_trailing_slash(url: &mut Url) {
    let path = url.path();

    if path.len() > 1 && path.ends_with('/') {
        let trimmed = path[..path.len() - 1].to_string();
        url.set_path(&trimmed);
    }
}

/// Sorts `url`'s query parameters alphabetically.
fn sort_query(url: &mut Url) {
    let Some(query) = url.query() else {
        return;
    };

    let mut items: Vec<&str> = query.split('&').collect();
    items.sort_unstable();
    let sorted = items.join("&");

    url.set_query(Some(&sorted));
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;
    use axum::http::StatusCode;
    use axum::http::header::LOCATION;

    use super::NormalizeParams;
    use super::normalize;

    #[tokio::test]
    async fn redirects_to_the_normalized_form_by_default() {
        let params = NormalizeParams {
            url: "HTTP://ExAmPle.COM:80/a/./b/../c/".to_string(),
            remove_host_dots: true,
            remove_trailing_slash: true,
            sort_query: true,
            remove_fragment: true,
        };

        let (status, headers) = normalize(Query(params)).await.unwrap();

        assert_eq!(status, StatusCode::MOVED_PERMANENTLY);
        assert_eq!(headers.get(LOCATION).unwrap(), "http://example.com/a/c");
    }

    #[tokio::test]
    async fn returns_ok_when_the_url_is_already_normalized() {
        let params = NormalizeParams {
            url: "http://example.com/path".to_string(),
            remove_host_dots: true,
            remove_trailing_slash: true,
            sort_query: true,
            remove_fragment: true,
        };

        let (status, headers) = normalize(Query(params)).await.unwrap();

        assert_eq!(status, StatusCode::OK);
        assert!(headers.get(LOCATION).is_none());
    }

    #[tokio::test]
    async fn removes_host_dots_when_enabled() {
        let params = NormalizeParams {
            url: "http://.example.com../path".to_string(),
            remove_host_dots: true,
            remove_trailing_slash: false,
            sort_query: false,
            remove_fragment: false,
        };

        let (_, headers) = normalize(Query(params)).await.unwrap();

        assert_eq!(headers.get(LOCATION).unwrap(), "http://example.com/path");
    }

    #[tokio::test]
    async fn keeps_host_dots_when_disabled() {
        let params = NormalizeParams {
            url: "http://example.com../path".to_string(),
            remove_host_dots: false,
            remove_trailing_slash: false,
            sort_query: false,
            remove_fragment: false,
        };

        let (status, _) = normalize(Query(params)).await.unwrap();

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn sorts_query_parameters_when_enabled() {
        let params = NormalizeParams {
            url: "http://example.com/path?c=3&a=1&b=2".to_string(),
            remove_host_dots: false,
            remove_trailing_slash: false,
            sort_query: true,
            remove_fragment: false,
        };

        let (_, headers) = normalize(Query(params)).await.unwrap();

        assert_eq!(
            headers.get(LOCATION).unwrap(),
            "http://example.com/path?a=1&b=2&c=3"
        );
    }

    #[tokio::test]
    async fn keeps_query_order_when_disabled() {
        let params = NormalizeParams {
            url: "http://example.com/path?c=3&a=1&b=2".to_string(),
            remove_host_dots: false,
            remove_trailing_slash: false,
            sort_query: false,
            remove_fragment: false,
        };

        let (status, _) = normalize(Query(params)).await.unwrap();

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn removes_fragment_when_enabled() {
        let params = NormalizeParams {
            url: "http://example.com/path#section".to_string(),
            remove_host_dots: false,
            remove_trailing_slash: false,
            sort_query: false,
            remove_fragment: true,
        };

        let (_, headers) = normalize(Query(params)).await.unwrap();

        assert_eq!(headers.get(LOCATION).unwrap(), "http://example.com/path");
    }

    #[tokio::test]
    async fn keeps_fragment_when_disabled() {
        let params = NormalizeParams {
            url: "http://example.com/path#section".to_string(),
            remove_host_dots: false,
            remove_trailing_slash: false,
            sort_query: false,
            remove_fragment: false,
        };

        let (status, _) = normalize(Query(params)).await.unwrap();

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn keeps_path_and_query_case_untouched() {
        let params = NormalizeParams {
            url: "http://example.com/Path?Key=Value".to_string(),
            remove_host_dots: true,
            remove_trailing_slash: true,
            sort_query: true,
            remove_fragment: true,
        };

        let (status, _) = normalize(Query(params)).await.unwrap();

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_an_unparseable_url() {
        let params = NormalizeParams {
            url: "not a url".to_string(),
            remove_host_dots: true,
            remove_trailing_slash: true,
            sort_query: true,
            remove_fragment: true,
        };

        let result = normalize(Query(params)).await;

        assert!(result.is_err());
    }
}
