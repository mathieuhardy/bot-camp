//! Header echo & arbitrary header injection endpoints.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::HeaderValue;

use crate::error::Result;

/// Returns every header received on the request, as JSON.
///
/// Header names are lower-cased (as per the HTTP spec, header names are
/// case-insensitive); a name repeated across several header lines (e.g.
/// `Accept-Language` sent twice) is grouped under a single key with all of
/// its values, in the order they were received.
///
/// # Returns
/// A JSON object mapping each received header name to the list of its
/// values.
pub async fn echo(headers: HeaderMap) -> Json<BTreeMap<String, Vec<String>>> {
    let mut received = BTreeMap::new();

    for (name, value) in &headers {
        received
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(String::from_utf8_lossy(value.as_bytes()).into_owned());
    }

    Json(received)
}

/// Sets arbitrary response headers from the query string, e.g.
/// `/headers/set?x-foo=bar&x-foo=baz` responds with two `x-foo` header
/// lines. Useful to simulate malformed or unusual header values (a bad
/// `Content-Type`, a stray `X-Robots-Tag`, etc.) that a crawler must
/// tolerate.
///
/// # Returns
/// The requested headers on a `200 OK` with an empty body, or an `Error`
/// if a name or value isn't valid for an HTTP header.
pub async fn set(Query(params): Query<Vec<(String, String)>>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    for (name, value) in params {
        let name = HeaderName::from_bytes(name.as_bytes())?;
        let value = HeaderValue::from_str(&value)?;
        headers.append(name, value);
    }

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;
    use axum::http::HeaderMap;
    use axum::http::HeaderValue;

    use super::echo;
    use super::set;

    #[tokio::test]
    async fn echo_groups_repeated_headers_under_one_key() {
        let mut headers = HeaderMap::new();
        headers.append("x-foo", HeaderValue::from_static("bar"));
        headers.append("x-foo", HeaderValue::from_static("baz"));

        let received = echo(headers).await.0;

        assert_eq!(
            received.get("x-foo"),
            Some(&vec!["bar".to_string(), "baz".to_string()])
        );
    }

    #[tokio::test]
    async fn set_appends_one_header_line_per_query_param() {
        let params = vec![
            ("x-foo".to_string(), "bar".to_string()),
            ("x-foo".to_string(), "baz".to_string()),
        ];

        let headers = set(Query(params)).await.unwrap();
        let values: Vec<_> = headers.get_all("x-foo").iter().collect();

        assert_eq!(values, vec!["bar", "baz"]);
    }

    #[tokio::test]
    async fn set_rejects_an_invalid_header_name() {
        let params = vec![("bad header".to_string(), "value".to_string())];

        let result = set(Query(params)).await;

        assert!(result.is_err());
    }
}
