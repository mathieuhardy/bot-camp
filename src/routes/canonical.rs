//! Canonical tag endpoint.

use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;

use crate::templates::PageContext;
use crate::templates::render_page;

/// Query parameters accepted by [`canonical`].
#[derive(Deserialize)]
pub(crate) struct CanonicalParams {
    /// The canonical URL to point at, absolute or relative — pass the
    /// page's own URL for a self-referential canonical, or another
    /// page's URL to test cross-page canonicalization.
    to: String,

    /// `og:url` meta tag content, to test a canonical/`og:url` conflict.
    #[serde(default)]
    og_url: Option<String>,

    /// Emit the canonical tag twice.
    #[serde(default)]
    duplicate: bool,

    /// Emit the canonical tag(s) in `<body>` instead of `<head>`, an
    /// invalid placement crawlers should reject.
    #[serde(default)]
    in_body: bool,
}

/// Returns an HTML page carrying a `<link rel="canonical">` tag, with
/// knobs for the classic tricky variants: self-referential vs
/// cross-page, relative vs absolute (both are just what `to` holds),
/// duplicated, out-of-`<head>`, and conflicting with `og:url`.
///
/// # Returns
/// `200 OK` with the rendered page.
pub async fn canonical(Query(params): Query<CanonicalParams>) -> Html<String> {
    let hrefs = if params.duplicate {
        vec![params.to.clone(), params.to]
    } else {
        vec![params.to]
    };

    let context = PageContext {
        title: "Canonical".to_string(),
        canonical_in_head: if params.in_body {
            Vec::new()
        } else {
            hrefs.clone()
        },
        canonical_in_body: if params.in_body { hrefs } else { Vec::new() },
        og_url: params.og_url,
        body: "Canonical tag test page.".to_string(),
        ..Default::default()
    };

    Html(render_page(context))
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;

    use super::CanonicalParams;
    use super::canonical;

    #[tokio::test]
    async fn renders_a_single_canonical_link_in_head_by_default() {
        let params = CanonicalParams {
            to: "/page".to_string(),
            og_url: None,
            duplicate: false,
            in_body: false,
        };

        let html = canonical(Query(params)).await.0;
        let head_end = html.find("</head>").unwrap();

        assert_eq!(html.matches(r#"href="/page""#).count(), 1);
        assert!(html[..head_end].contains(r#"href="/page""#));
    }

    #[tokio::test]
    async fn duplicates_the_canonical_link_when_requested() {
        let params = CanonicalParams {
            to: "/page".to_string(),
            og_url: None,
            duplicate: true,
            in_body: false,
        };

        let html = canonical(Query(params)).await.0;

        assert_eq!(html.matches(r#"href="/page""#).count(), 2);
    }

    #[tokio::test]
    async fn moves_the_canonical_link_into_the_body_when_requested() {
        let params = CanonicalParams {
            to: "/page".to_string(),
            og_url: None,
            duplicate: false,
            in_body: true,
        };

        let html = canonical(Query(params)).await.0;
        let head_end = html.find("</head>").unwrap();

        assert!(!html[..head_end].contains(r#"href="/page""#));
        assert!(html[head_end..].contains(r#"href="/page""#));
    }

    #[tokio::test]
    async fn embeds_the_og_url_meta_tag() {
        let params = CanonicalParams {
            to: "/page".to_string(),
            og_url: Some("/other".to_string()),
            duplicate: false,
            in_body: false,
        };

        let html = canonical(Query(params)).await.0;

        assert!(html.contains(r#"<meta property="og:url" content="/other">"#));
    }
}
