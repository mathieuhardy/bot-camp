//! Charset declaration mismatches and double-encoded content.

use axum::body::Body;
use axum::extract::Query;
use axum::http::Response;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::templates::PageContext;
use crate::templates::escape_html;
use crate::templates::render_page;

/// Query parameters accepted by [`encoding`].
#[derive(Deserialize)]
pub(crate) struct EncodingParams {
    /// The page's body text, before any encoding is applied. Defaults to
    /// a string mixing accented Latin characters, CJK characters, and an
    /// ampersand, to exercise multi-lingual content and double-encoding
    /// at once.
    #[serde(default = "default_text")]
    text: String,

    /// Charset declared in the `Content-Type` response header.
    #[serde(default = "default_charset")]
    content_type_charset: String,

    /// Charset declared in a `<meta charset>` tag, if any — independent
    /// of `content_type_charset`, to test a header/meta mismatch.
    #[serde(default)]
    meta_charset: Option<String>,

    /// HTML-entity-encode `text` twice instead of once, to simulate the
    /// classic double-encoding bug (e.g. `&` becoming `&amp;amp;`).
    #[serde(default)]
    double_encode: bool,
}

/// Default value for [`EncodingParams::text`].
fn default_text() -> String {
    "Café & Résumé 日本語".to_string()
}

/// Default value for [`EncodingParams::content_type_charset`].
fn default_charset() -> String {
    "utf-8".to_string()
}

/// Returns an HTML page whose declared charsets (`Content-Type` header
/// and `<meta charset>` tag) are independently controllable, and whose
/// body text can be double HTML-entity-encoded — to test how a crawler
/// handles a mismatch between declared and actual encoding, and the
/// classic double-encoding bug.
///
/// # Returns
/// `200 OK` with the rendered page and a `Content-Type` header set to
/// `content_type_charset`.
pub async fn encoding(Query(params): Query<EncodingParams>) -> impl IntoResponse {
    let body = if params.double_encode {
        escape_html(&params.text)
    } else {
        params.text
    };

    let context = PageContext {
        charset: params.meta_charset,
        body,
        ..Default::default()
    };

    let html = render_page(context);
    let content_type = format!("text/html; charset={}", params.content_type_charset);

    Response::builder()
        .header(CONTENT_TYPE, content_type)
        .body(Body::from(html))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use axum::extract::Query;
    use axum::response::IntoResponse;

    use super::EncodingParams;
    use super::encoding;

    fn params() -> EncodingParams {
        EncodingParams {
            text: "Café & Résumé 日本語".to_string(),
            content_type_charset: "utf-8".to_string(),
            meta_charset: None,
            double_encode: false,
        }
    }

    #[tokio::test]
    async fn declares_the_requested_content_type_charset() {
        let response = encoding(Query(params())).await.into_response();

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn can_declare_a_content_type_charset_independent_of_meta_charset() {
        let response = encoding(Query(EncodingParams {
            content_type_charset: "iso-8859-1".to_string(),
            meta_charset: Some("utf-8".to_string()),
            ..params()
        }))
        .await
        .into_response();

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/html; charset=iso-8859-1"
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains(r#"<meta charset="utf-8">"#));
    }

    #[tokio::test]
    async fn serves_the_text_as_is_by_default() {
        let response = encoding(Query(params())).await.into_response();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("Café &amp; Résumé 日本語"));
    }

    #[tokio::test]
    async fn double_encodes_the_text_when_requested() {
        let response = encoding(Query(EncodingParams {
            double_encode: true,
            ..params()
        }))
        .await
        .into_response();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("Café &amp;amp; Résumé 日本語"));
    }
}
