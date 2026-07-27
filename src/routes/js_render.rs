//! Deferred, JavaScript-injected content.

use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;

use crate::templates::PageContext;
use crate::templates::render_page;

/// Query parameters accepted by [`js_render`].
#[derive(Deserialize)]
pub(crate) struct JsRenderParams {
    /// Text injected into the page body after `delay_ms`, via
    /// JavaScript.
    #[serde(default)]
    text: Option<String>,

    /// `document.title` set after `delay_ms`, via JavaScript.
    #[serde(default)]
    title: Option<String>,

    /// `<link rel="canonical">` href injected into `<head>` after
    /// `delay_ms`, via JavaScript.
    #[serde(default)]
    canonical: Option<String>,

    /// `<meta>` tag name injected into `<head>` after `delay_ms`, via
    /// JavaScript. Only injected if `meta_content` is also given.
    #[serde(default)]
    meta_name: Option<String>,

    /// `<meta>` tag content, paired with `meta_name`.
    #[serde(default)]
    meta_content: Option<String>,

    /// Delay, in milliseconds, before the JavaScript mutates the page.
    #[serde(default)]
    delay_ms: u64,
}

/// Returns an HTML page whose initial, server-rendered markup carries
/// none of `text`, `title`, `canonical`, or the `meta_name`/`meta_content`
/// pair — each is injected into the DOM via JavaScript after `delay_ms`
/// instead. Useful to check whether a crawler executes JavaScript before
/// extracting these signals, or only sees the initial HTML.
///
/// # Returns
/// `200 OK` with the rendered page.
pub async fn js_render(Query(params): Query<JsRenderParams>) -> Html<String> {
    let mut statements = Vec::new();

    if let Some(text) = &params.text {
        statements.push(format!(
            "document.getElementById('js-content').textContent = {};",
            js_string_literal(text)
        ));
    }

    if let Some(title) = &params.title {
        statements.push(format!("document.title = {};", js_string_literal(title)));
    }

    if let Some(canonical) = &params.canonical {
        statements.push(format!(
            "var link = document.createElement('link'); \
             link.rel = 'canonical'; \
             link.href = {}; \
             document.head.appendChild(link);",
            js_string_literal(canonical)
        ));
    }

    if let (Some(name), Some(content)) = (&params.meta_name, &params.meta_content) {
        statements.push(format!(
            "var meta = document.createElement('meta'); \
             meta.name = {}; \
             meta.content = {}; \
             document.head.appendChild(meta);",
            js_string_literal(name),
            js_string_literal(content)
        ));
    }

    let deferred_script = (!statements.is_empty()).then(|| {
        format!(
            "setTimeout(function() {{ {} }}, {});",
            statements.join(" "),
            params.delay_ms
        )
    });

    let context = PageContext {
        deferred_script,
        ..Default::default()
    };

    Html(render_page(context))
}

/// Encodes `value` as a double-quoted JavaScript string literal, safe to
/// interpolate into a `<script>` block: backslashes, double quotes, and
/// newlines are escaped, and `<` is unicode-escaped so the value can't
/// break out of the surrounding `<script>` tag.
fn js_string_literal(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');

    for c in value.chars() {
        match c {
            '\\' => literal.push_str("\\\\"),
            '"' => literal.push_str("\\\""),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '<' => literal.push_str("\\u003C"),
            _ => literal.push(c),
        }
    }

    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;

    use super::JsRenderParams;
    use super::js_render;

    fn params() -> JsRenderParams {
        JsRenderParams {
            text: None,
            title: None,
            canonical: None,
            meta_name: None,
            meta_content: None,
            delay_ms: 0,
        }
    }

    #[tokio::test]
    async fn omits_every_signal_from_the_initial_html_by_default() {
        let html = js_render(Query(params())).await.0;

        assert!(!html.contains("<title>"));
        assert!(!html.contains("rel=\"canonical\""));
        assert!(!html.contains("<script>"));
    }

    #[tokio::test]
    async fn injects_text_via_a_deferred_script() {
        let html = js_render(Query(JsRenderParams {
            text: Some("hello".to_string()),
            ..params()
        }))
        .await
        .0;

        assert!(html.contains(r#"id="js-content""#));
        assert!(html.contains("document.getElementById('js-content').textContent = \"hello\";"));
    }

    #[tokio::test]
    async fn injects_the_title_via_a_deferred_script() {
        let html = js_render(Query(JsRenderParams {
            title: Some("Injected".to_string()),
            ..params()
        }))
        .await
        .0;

        assert!(html.contains("document.title = \"Injected\";"));
    }

    #[tokio::test]
    async fn injects_the_canonical_via_a_deferred_script() {
        let html = js_render(Query(JsRenderParams {
            canonical: Some("/page".to_string()),
            ..params()
        }))
        .await
        .0;

        assert!(html.contains("link.href = \"/page\";"));
    }

    #[tokio::test]
    async fn injects_the_meta_tag_only_when_both_name_and_content_are_given() {
        let html = js_render(Query(JsRenderParams {
            meta_name: Some("description".to_string()),
            ..params()
        }))
        .await
        .0;

        assert!(!html.contains("appendChild(meta)"));

        let html = js_render(Query(JsRenderParams {
            meta_name: Some("description".to_string()),
            meta_content: Some("Test page".to_string()),
            ..params()
        }))
        .await
        .0;

        assert!(html.contains("meta.name = \"description\";"));
        assert!(html.contains("meta.content = \"Test page\";"));
    }

    #[tokio::test]
    async fn honors_the_configured_delay() {
        let html = js_render(Query(JsRenderParams {
            text: Some("hi".to_string()),
            delay_ms: 2000,
            ..params()
        }))
        .await
        .0;

        assert!(html.contains("}, 2000);"));
    }

    #[tokio::test]
    async fn escapes_a_value_that_would_otherwise_break_out_of_the_script() {
        let html = js_render(Query(JsRenderParams {
            text: Some("</script><script>alert(1)</script>".to_string()),
            ..params()
        }))
        .await
        .0;

        assert!(!html.contains("</script><script>alert(1)"));
    }
}
