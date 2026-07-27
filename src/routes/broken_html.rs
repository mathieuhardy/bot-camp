//! Deliberately malformed HTML.

use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;

use crate::templates::PageContext;
use crate::templates::render_page;

/// Query parameters accepted by [`broken_html`].
#[derive(Deserialize)]
pub(crate) struct BrokenHtmlParams {
    /// Raw markup inserted verbatim (not HTML-escaped) into `<head>` —
    /// craft an unclosed tag, or an element that isn't valid inside
    /// `<head>`, to test how a crawler's parser recovers.
    #[serde(default)]
    head: Option<String>,

    /// Raw markup inserted verbatim (not HTML-escaped) into `<body>`.
    #[serde(default)]
    body: Option<String>,
}

/// Returns an HTML page with `head`/`body` spliced in verbatim, letting
/// you construct any malformed markup you want to test — an unclosed
/// tag inside `<head>`, a non-head element misplaced in `<head>`, a
/// `<link>` inside `<body>`, or anything else a real parser would need
/// to recover from.
///
/// # Returns
/// `200 OK` with the rendered page.
pub async fn broken_html(Query(params): Query<BrokenHtmlParams>) -> Html<String> {
    let context = PageContext {
        raw_head: params.head,
        raw_body: params.body,
        ..Default::default()
    };

    Html(render_page(context))
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;

    use super::BrokenHtmlParams;
    use super::broken_html;

    #[tokio::test]
    async fn renders_the_default_skeleton_unchanged_when_omitted() {
        let html = broken_html(Query(BrokenHtmlParams {
            head: None,
            body: None,
        }))
        .await
        .0;

        assert!(html.contains("<head>"));
        assert!(html.contains("<body>"));
    }

    #[tokio::test]
    async fn splices_raw_head_markup_unescaped() {
        let html = broken_html(Query(BrokenHtmlParams {
            head: Some("<p>not valid in head</p>".to_string()),
            body: None,
        }))
        .await
        .0;

        let head_end = html.find("</head>").unwrap();
        assert!(html[..head_end].contains("<p>not valid in head</p>"));
    }

    #[tokio::test]
    async fn splices_raw_body_markup_unescaped() {
        let html = broken_html(Query(BrokenHtmlParams {
            head: None,
            body: Some(r#"<link rel="stylesheet" href="/x.css">"#.to_string()),
        }))
        .await
        .0;

        let head_end = html.find("</head>").unwrap();
        assert!(html[head_end..].contains(r#"<link rel="stylesheet" href="/x.css">"#));
    }
}
