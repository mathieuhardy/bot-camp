//! `robots.txt` and meta-robots endpoints.

use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::response::Html;
use serde::Deserialize;

use crate::error::Result;
use crate::state::AppState;
use crate::templates::PageContext;
use crate::templates::render_page;

/// Query parameters accepted by [`robots_meta`].
#[derive(Deserialize)]
pub(crate) struct MetaParams {
    /// `meta name="robots"` content attribute, verbatim — e.g.
    /// `noindex,nofollow`, in whatever casing or combination you want to
    /// test.
    directives: String,

    /// Optional `X-Robots-Tag` response header value, to test a
    /// meta-robots/header conflict.
    #[serde(default)]
    x_robots_tag: Option<String>,

    /// Emit the meta tag twice.
    #[serde(default)]
    duplicate: bool,
}

/// Returns the `/robots.txt` contents currently stored in `state`.
///
/// # Returns
/// `200 OK` with the stored text — the built-in default until
/// [`set_robots_txt`] is called.
pub async fn robots_txt(State(state): State<AppState>) -> String {
    state.robots_txt.read().await.clone()
}

/// Overwrites the contents served by [`robots_txt`] with the request
/// body, verbatim. Since a crawler always requests `/robots.txt` with no
/// query string, this is the only way to steer its content — write
/// whichever edge case you want to test (empty lines, mixed casing,
/// duplicated directives, several `User-agent` groups, an `Allow` longer
/// or shorter than its `Disallow`) directly into the body.
///
/// # Returns
/// `200 OK` once the new content is stored.
pub async fn set_robots_txt(State(state): State<AppState>, body: String) {
    *state.robots_txt.write().await = body;
}

/// Returns an HTML page carrying a `<meta name="robots">` tag, and
/// optionally an `X-Robots-Tag` response header, to test the classic
/// meta-robots edge cases: directive combinations, case variations,
/// duplication, and conflicts between the meta tag and the header.
///
/// # Returns
/// `200 OK` with the rendered page, or an `Error` if `x_robots_tag` isn't
/// a valid header value.
pub async fn robots_meta(Query(params): Query<MetaParams>) -> Result<(HeaderMap, Html<String>)> {
    let contents = if params.duplicate {
        vec![params.directives.clone(), params.directives]
    } else {
        vec![params.directives]
    };

    let context = PageContext {
        titles: vec!["Robots meta".to_string()],
        meta_robots: contents,
        body: "Robots meta tag test page.".to_string(),
        ..Default::default()
    };

    let mut headers = HeaderMap::new();
    if let Some(x_robots_tag) = params.x_robots_tag {
        headers.insert(
            HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_str(&x_robots_tag)?,
        );
    }

    Ok((headers, Html(render_page(context))))
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;
    use axum::extract::State;

    use super::MetaParams;
    use super::robots_meta;
    use super::robots_txt;
    use super::set_robots_txt;
    use crate::state::AppState;

    #[tokio::test]
    async fn robots_txt_serves_the_default_content_initially() {
        let state = AppState::default();

        let body = robots_txt(State(state)).await;

        assert_eq!(body, "User-agent: *\nAllow: /\n");
    }

    #[tokio::test]
    async fn set_robots_txt_overwrites_what_robots_txt_serves() {
        let state = AppState::default();

        set_robots_txt(State(state.clone()), "Disallow: /private\n".to_string()).await;
        let body = robots_txt(State(state)).await;

        assert_eq!(body, "Disallow: /private\n");
    }

    #[tokio::test]
    async fn robots_meta_renders_a_single_tag_in_head_by_default() {
        let params = MetaParams {
            directives: "noindex,nofollow".to_string(),
            x_robots_tag: None,
            duplicate: false,
        };

        let (headers, html) = robots_meta(Query(params)).await.unwrap();
        let html = html.0;
        let head_end = html.find("</head>").unwrap();

        assert_eq!(html.matches(r#"content="noindex,nofollow""#).count(), 1);
        assert!(html[..head_end].contains(r#"content="noindex,nofollow""#));
        assert!(headers.get("x-robots-tag").is_none());
    }

    #[tokio::test]
    async fn robots_meta_duplicates_the_tag_when_requested() {
        let params = MetaParams {
            directives: "noindex".to_string(),
            x_robots_tag: None,
            duplicate: true,
        };

        let (_, html) = robots_meta(Query(params)).await.unwrap();

        assert_eq!(html.0.matches(r#"content="noindex""#).count(), 2);
    }

    #[tokio::test]
    async fn robots_meta_sets_the_x_robots_tag_header() {
        let params = MetaParams {
            directives: "index".to_string(),
            x_robots_tag: Some("noindex".to_string()),
            duplicate: false,
        };

        let (headers, _) = robots_meta(Query(params)).await.unwrap();

        assert_eq!(headers.get("x-robots-tag").unwrap(), "noindex");
    }
}
