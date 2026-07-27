//! Controlled on-page content: title, H1, and body/word count.

use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;

use crate::templates::PageContext;
use crate::templates::escape_html;
use crate::templates::render_page;

/// Query parameters accepted by [`content`].
#[derive(Deserialize)]
pub(crate) struct ContentParams {
    /// The page's `<title>` contents. Omitted entirely renders no
    /// `<title>` tag at all — distinct from passing an empty string,
    /// which renders `<title></title>`.
    #[serde(default)]
    title: Option<String>,

    /// Emit the `<title>` tag twice. Only meaningful if `title` is set.
    #[serde(default)]
    duplicate_title: bool,

    /// The page's `<h1>` contents. Omitted entirely renders no `<h1>`
    /// tag at all.
    #[serde(default)]
    h1: Option<String>,

    /// Emit the `<h1>` tag twice. Only meaningful if `h1` is set.
    #[serde(default)]
    duplicate_h1: bool,

    /// Number of filler words (`word0 word1 ...`) to use as the body,
    /// if `body` isn't given.
    #[serde(default)]
    word_count: Option<u32>,

    /// The page's body text, verbatim. Requesting this route with the
    /// same `body` from two different URLs is how you simulate
    /// duplicate content across two pages.
    #[serde(default)]
    body: Option<String>,

    /// URL for a link hidden from real users (positioned off-screen),
    /// appended to the body — point it at `/honeypot/...` to bait a
    /// crawler that blindly follows every link regardless of
    /// visibility.
    #[serde(default)]
    hidden_link: Option<String>,
}

/// Returns an HTML page with controllable `<title>`, `<h1>`, and body
/// content, to test the classic on-page signals a crawler extracts:
/// missing/empty/duplicated titles, missing/duplicated H1s, a precise
/// word count, and duplicate content across pages.
///
/// # Returns
/// `200 OK` with the rendered page.
pub async fn content(Query(params): Query<ContentParams>) -> Html<String> {
    let body = params
        .body
        .unwrap_or_else(|| filler_words(params.word_count.unwrap_or(0)));

    let context = PageContext {
        titles: repeated(params.title, params.duplicate_title),
        h1: repeated(params.h1, params.duplicate_h1),
        body,
        raw_body: params.hidden_link.map(|href| hidden_link_markup(&href)),
        ..Default::default()
    };

    Html(render_page(context))
}

/// Renders `href` as a link positioned off-screen — invisible to a real
/// user, but present in the HTML for a crawler that follows every
/// `href` regardless of visibility.
fn hidden_link_markup(href: &str) -> String {
    format!(
        r#"<a href="{}" style="position:absolute;left:-9999px" aria-hidden="true">hidden link</a>"#,
        escape_html(href)
    )
}

/// Wraps `value` into a `Vec`, duplicated if `duplicate` is set, or an
/// empty `Vec` if `value` is absent.
fn repeated(value: Option<String>, duplicate: bool) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };

    if duplicate {
        vec![value.clone(), value]
    } else {
        vec![value]
    }
}

/// Generates `count` space-separated filler words (`word0 word1 ...`).
fn filler_words(count: u32) -> String {
    (0..count)
        .map(|i| format!("word{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;

    use super::ContentParams;
    use super::content;

    fn params() -> ContentParams {
        ContentParams {
            title: None,
            duplicate_title: false,
            h1: None,
            duplicate_h1: false,
            word_count: None,
            body: None,
            hidden_link: None,
        }
    }

    #[tokio::test]
    async fn renders_no_title_when_omitted() {
        let html = content(Query(params())).await.0;

        assert!(!html.contains("<title>"));
    }

    #[tokio::test]
    async fn renders_an_empty_title_distinctly_from_a_missing_one() {
        let html = content(Query(ContentParams {
            title: Some(String::new()),
            ..params()
        }))
        .await
        .0;

        assert!(html.contains("<title></title>"));
    }

    #[tokio::test]
    async fn duplicates_the_title_when_requested() {
        let html = content(Query(ContentParams {
            title: Some("Page".to_string()),
            duplicate_title: true,
            ..params()
        }))
        .await
        .0;

        assert_eq!(html.matches("<title>Page</title>").count(), 2);
    }

    #[tokio::test]
    async fn renders_no_h1_when_omitted() {
        let html = content(Query(params())).await.0;

        assert!(!html.contains("<h1>"));
    }

    #[tokio::test]
    async fn duplicates_the_h1_when_requested() {
        let html = content(Query(ContentParams {
            h1: Some("Heading".to_string()),
            duplicate_h1: true,
            ..params()
        }))
        .await
        .0;

        assert_eq!(html.matches("<h1>Heading</h1>").count(), 2);
    }

    #[tokio::test]
    async fn generates_the_requested_word_count() {
        let html = content(Query(ContentParams {
            word_count: Some(5),
            ..params()
        }))
        .await
        .0;

        assert!(html.contains("word0 word1 word2 word3 word4"));
    }

    #[tokio::test]
    async fn body_takes_precedence_over_word_count() {
        let html = content(Query(ContentParams {
            body: Some("exact body text".to_string()),
            word_count: Some(5),
            ..params()
        }))
        .await
        .0;

        assert!(html.contains("exact body text"));
        assert!(!html.contains("word0"));
    }

    #[tokio::test]
    async fn renders_no_hidden_link_when_omitted() {
        let html = content(Query(params())).await.0;

        assert!(!html.contains("<a "));
    }

    #[tokio::test]
    async fn renders_an_off_screen_hidden_link_when_requested() {
        let html = content(Query(ContentParams {
            hidden_link: Some("/honeypot/trap".to_string()),
            ..params()
        }))
        .await
        .0;

        assert!(html.contains(r#"href="/honeypot/trap""#));
        assert!(html.contains("position:absolute;left:-9999px"));
    }
}
