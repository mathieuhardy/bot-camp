//! Shared HTML page rendering for content-based test endpoints.

use minijinja::AutoEscape;
use minijinja::Environment;
use serde::Serialize;

/// Name under which the shared page skeleton is registered.
const TEMPLATE_NAME: &str = "page.html";

/// The shared HTML page skeleton: a title, zero or more canonical links
/// (in the head and/or the body), an optional `og:url` meta tag, an
/// optional meta-refresh tag, a body, an optional deferred script, an
/// optional charset meta tag, and raw markup spliced into the head/body.
const TEMPLATE_SOURCE: &str = include_str!("../templates/page.html");

/// Values interpolated into the shared page skeleton.
#[derive(Default, Serialize)]
pub(crate) struct PageContext {
    /// `<title>` contents to render, one tag per entry — zero to omit
    /// the tag entirely.
    pub(crate) titles: Vec<String>,

    /// Canonical link hrefs to render inside `<head>`.
    pub(crate) canonical_in_head: Vec<String>,

    /// Canonical link hrefs to render inside `<body>`, to simulate an
    /// invalid placement.
    pub(crate) canonical_in_body: Vec<String>,

    /// `og:url` meta tag content, if any.
    pub(crate) og_url: Option<String>,

    /// `meta http-equiv="refresh"` content attribute, if any.
    pub(crate) refresh: Option<String>,

    /// `meta name="robots"` content attribute values to render, one tag
    /// per entry.
    pub(crate) meta_robots: Vec<String>,

    /// `<h1>` contents to render, one tag per entry — zero to omit the
    /// tag entirely.
    pub(crate) h1: Vec<String>,

    /// Body text.
    pub(crate) body: String,

    /// JavaScript statements to run, inserted verbatim (not
    /// HTML-escaped, since it's JS source rather than HTML text) inside
    /// a `<script>` tag, alongside an empty `#js-content` element for
    /// that script to populate.
    pub(crate) deferred_script: Option<String>,

    /// `<meta charset>` value, if any.
    pub(crate) charset: Option<String>,

    /// Raw markup inserted verbatim (not HTML-escaped) into `<head>`,
    /// to construct deliberately malformed HTML.
    pub(crate) raw_head: Option<String>,

    /// Raw markup inserted verbatim (not HTML-escaped) into `<body>`,
    /// to construct deliberately malformed HTML.
    pub(crate) raw_body: Option<String>,
}

/// Renders the shared HTML skeleton with `context`, HTML-escaping every
/// interpolated value.
///
/// MiniJinja's built-in HTML autoescaper also escapes `/` (as
/// `&#x2f;`), which would mangle every URL rendered by this tool. Escape
/// the five characters that actually matter ourselves instead, and turn
/// autoescaping off.
///
/// # Returns
/// The rendered HTML.
pub(crate) fn render_page(context: PageContext) -> String {
    let context = PageContext {
        titles: context.titles.iter().map(|s| escape_html(s)).collect(),
        canonical_in_head: context
            .canonical_in_head
            .iter()
            .map(|s| escape_html(s))
            .collect(),
        canonical_in_body: context
            .canonical_in_body
            .iter()
            .map(|s| escape_html(s))
            .collect(),
        og_url: context.og_url.as_deref().map(escape_html),
        refresh: context.refresh.as_deref().map(escape_html),
        meta_robots: context.meta_robots.iter().map(|s| escape_html(s)).collect(),
        h1: context.h1.iter().map(|s| escape_html(s)).collect(),
        body: escape_html(&context.body),
        deferred_script: context.deferred_script,
        charset: context.charset.as_deref().map(escape_html),
        raw_head: context.raw_head,
        raw_body: context.raw_body,
    };

    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::None);
    env.add_template(TEMPLATE_NAME, TEMPLATE_SOURCE)
        .expect("bundled template is valid MiniJinja syntax");

    env.get_template(TEMPLATE_NAME)
        .expect("template was just registered under TEMPLATE_NAME")
        .render(context)
        .expect("rendering a fully-populated PageContext cannot fail")
}

/// Escapes the five characters that are unsafe to interpolate into HTML
/// text or a quoted attribute value: `&`, `<`, `>`, `"`, `'`.
pub(crate) fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::PageContext;
    use super::render_page;

    #[test]
    fn escapes_interpolated_values() {
        let context = PageContext {
            titles: vec!["<script>".to_string()],
            ..Default::default()
        };

        let html = render_page(context);

        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn renders_canonical_links_in_head_and_body() {
        let context = PageContext {
            canonical_in_head: vec!["/a".to_string()],
            canonical_in_body: vec!["/b".to_string()],
            ..Default::default()
        };

        let html = render_page(context);
        let head_end = html.find("</head>").unwrap();

        assert!(html[..head_end].contains(r#"href="/a""#));
        assert!(html[head_end..].contains(r#"href="/b""#));
    }
}
