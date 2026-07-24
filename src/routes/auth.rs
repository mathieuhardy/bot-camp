//! HTTP Basic Auth endpoint.

use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::header::WWW_AUTHENTICATE;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// Username expected in the `Authorization` header. Published here (and
/// in the API docs) on purpose: the point is to test a crawler's
/// known-good path, not to make it guess credentials.
const USERNAME: &str = "bot-camp";

/// Password expected in the `Authorization` header, see `USERNAME`.
const PASSWORD: &str = "bot-camp";

/// Realm advertised in the `WWW-Authenticate` challenge.
const REALM: &str = "bot-camp";

/// Returns `200 OK` when the request carries the expected HTTP Basic
/// credentials, or challenges for them otherwise.
///
/// # Returns
/// `200 OK` with an empty body if the `Authorization` header holds
/// `USERNAME:PASSWORD`; `401 Unauthorized` with a `WWW-Authenticate`
/// challenge otherwise.
pub async fn basic(headers: HeaderMap) -> (StatusCode, HeaderMap) {
    if has_valid_credentials(&headers) {
        return (StatusCode::OK, HeaderMap::new());
    }

    let mut challenge = HeaderMap::new();
    challenge.insert(
        WWW_AUTHENTICATE,
        HeaderValue::from_str(&format!("Basic realm=\"{REALM}\"")).unwrap(),
    );

    (StatusCode::UNAUTHORIZED, challenge)
}

/// Checks whether `headers` carries a valid `Authorization: Basic ...`
/// value.
fn has_valid_credentials(headers: &HeaderMap) -> bool {
    let Some((username, password)) = decode_credentials(headers) else {
        return false;
    };

    username == USERNAME && password == PASSWORD
}

/// Decodes the `username:password` pair from the request's
/// `Authorization` header, if present and well-formed.
fn decode_credentials(headers: &HeaderMap) -> Option<(String, String)> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;
    let decoded = STANDARD.decode(encoded).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (username, password) = decoded.split_once(':')?;

    Some((username.to_string(), password.to_string()))
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;
    use axum::http::HeaderValue;
    use axum::http::StatusCode;
    use axum::http::header::AUTHORIZATION;
    use axum::http::header::WWW_AUTHENTICATE;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    use super::basic;

    fn authorization_header(username: &str, password: &str) -> HeaderValue {
        let credentials = STANDARD.encode(format!("{username}:{password}"));

        HeaderValue::from_str(&format!("Basic {credentials}")).unwrap()
    }

    #[tokio::test]
    async fn accepts_the_expected_credentials() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization_header("bot-camp", "bot-camp"));

        let (status, _) = basic(headers).await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn challenges_a_missing_authorization_header() {
        let (status, headers) = basic(HeaderMap::new()).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(headers.contains_key(WWW_AUTHENTICATE));
    }

    #[tokio::test]
    async fn rejects_wrong_credentials() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization_header("bot-camp", "wrong"));

        let (status, _) = basic(headers).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_a_malformed_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic not-base64!"));

        let (status, _) = basic(headers).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
