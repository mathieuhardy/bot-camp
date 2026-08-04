//! URL discovery test page: a fixed, deterministic set of target URLs
//! spread across every common (and not-so-common) HTML mechanism a
//! crawler might need to extract links from. Compare what your crawler
//! actually extracts against the `/discovery/target/{n}` URLs this page
//! always generates, in the same deterministic order, for a given
//! `count`/`forms`.

use axum::extract::Path;
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::http::header::HOST;
use axum::response::Html;
use serde::Deserialize;

use crate::templates::PageContext;
use crate::templates::escape_html;
use crate::templates::render_page;

/// Every HTML mechanism `/discovery` can carry a target URL through.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Form {
    Anchor,
    Link,
    Image,
    Script,
    Comment,
    JsString,
    CssUrl,
    ProtocolRelative,
    Action,
    Iframe,
    Area,
}

impl Form {
    /// Every supported form, in the order used when cycling through them.
    const ALL: [Form; 11] = [
        Form::Anchor,
        Form::Link,
        Form::Image,
        Form::Script,
        Form::Comment,
        Form::JsString,
        Form::CssUrl,
        Form::ProtocolRelative,
        Form::Action,
        Form::Iframe,
        Form::Area,
    ];

    /// Parses a form's `forms` query param name, `None` if unrecognized.
    fn parse(name: &str) -> Option<Form> {
        match name {
            "a" => Some(Form::Anchor),
            "link" => Some(Form::Link),
            "img" => Some(Form::Image),
            "script" => Some(Form::Script),
            "comment" => Some(Form::Comment),
            "js" => Some(Form::JsString),
            "css" => Some(Form::CssUrl),
            "protocol_relative" => Some(Form::ProtocolRelative),
            "form" => Some(Form::Action),
            "iframe" => Some(Form::Iframe),
            "area" => Some(Form::Area),
            _ => None,
        }
    }
}

/// Query parameters accepted by [`discovery`].
#[derive(Deserialize)]
pub(crate) struct DiscoveryParams {
    /// How many distinct target URLs to generate. Defaults to one per
    /// supported form.
    #[serde(default = "default_count")]
    count: u32,

    /// Comma-separated subset of forms to cycle through — `a`, `link`,
    /// `img`, `script`, `comment`, `js`, `css`, `protocol_relative`,
    /// `form`, `iframe`, `area`. Defaults to every form; unknown names
    /// are dropped, and an empty/all-unknown list also falls back to
    /// every form.
    #[serde(default)]
    forms: Option<String>,
}

fn default_count() -> u32 {
    Form::ALL.len() as u32
}

/// Returns an HTML page listing `count` deterministic target URLs
/// (`/discovery/target/0`, `/discovery/target/1`, ...), cycling through
/// `forms` to decide which HTML mechanism carries each one.
///
/// # Returns
/// `200 OK` with the rendered page.
pub async fn discovery(headers: HeaderMap, Query(params): Query<DiscoveryParams>) -> Html<String> {
    let forms = parse_forms(params.forms.as_deref());
    let host = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("localhost:3000");

    let mut head_extra = String::new();
    let mut body_extra = String::new();

    for n in 0..params.count {
        let url = format!("/discovery/target/{n}");
        let form = forms[(n as usize) % forms.len()];
        render_target(form, n, &url, host, &mut head_extra, &mut body_extra);
    }

    let context = PageContext {
        titles: vec!["URL discovery".to_string()],
        h1: vec!["URL discovery".to_string()],
        raw_head: (!head_extra.is_empty()).then_some(head_extra),
        raw_body: (!body_extra.is_empty()).then_some(body_extra),
        ..Default::default()
    };

    Html(render_page(context))
}

/// Parses `forms` (the raw query param value) into the list to cycle
/// through, falling back to every form if it's absent, empty, or every
/// name in it is unrecognized.
fn parse_forms(forms: Option<&str>) -> Vec<Form> {
    let parsed = forms
        .map(|list| list.split(',').filter_map(Form::parse).collect::<Vec<_>>())
        .unwrap_or_default();

    if parsed.is_empty() {
        Form::ALL.to_vec()
    } else {
        parsed
    }
}

/// Appends the markup for one target URL, in the mechanism `form`
/// dictates, to `head`/`body` as appropriate.
fn render_target(form: Form, n: u32, url: &str, host: &str, head: &mut String, body: &mut String) {
    match form {
        Form::Anchor => {
            body.push_str(&format!("<a href=\"{url}\">target {n}</a>\n"));
        }

        Form::Link => {
            head.push_str(&format!("<link rel=\"alternate\" href=\"{url}\">\n"));
        }

        Form::Image => {
            body.push_str(&format!("<img src=\"{url}\" alt=\"target {n}\">\n"));
        }

        Form::Script => {
            body.push_str(&format!("<script src=\"{url}\"></script>\n"));
        }

        Form::Comment => {
            body.push_str(&format!("<!-- <a href=\"{url}\">target {n}</a> -->\n"));
        }

        Form::JsString => {
            body.push_str(&format!("<script>var target_{n} = \"{url}\";</script>\n"));
        }

        Form::CssUrl => {
            body.push_str(&format!(
                "<style>.target-{n} {{ background-image: url({url}); }}</style>\n"
            ));
        }

        Form::ProtocolRelative => {
            let host = escape_html(host);
            body.push_str(&format!(
                "<a href=\"//{host}{url}\">target {n} (protocol-relative)</a>\n"
            ));
        }

        Form::Action => {
            body.push_str(&format!("<form action=\"{url}\"></form>\n"));
        }

        Form::Iframe => {
            body.push_str(&format!("<iframe src=\"{url}\"></iframe>\n"));
        }

        Form::Area => {
            body.push_str(&format!(
                "<map><area href=\"{url}\" shape=\"rect\" coords=\"0,0,10,10\"></map>\n"
            ));
        }
    }
}

/// The target a discovered URL actually points at — always `200 OK`, so
/// following one confirms whether the crawler both extracted *and*
/// fetched it (check bot-camp's request logs to tell those apart).
///
/// # Returns
/// `200 OK` with a short acknowledgement.
pub async fn target(Path(n): Path<u32>) -> String {
    format!("ok: /discovery/target/{n}")
}

#[cfg(test)]
mod tests {
    use super::Form;
    use super::parse_forms;

    #[test]
    fn parse_forms_defaults_to_every_form_when_absent() {
        assert_eq!(parse_forms(None).len(), Form::ALL.len());
    }

    #[test]
    fn parse_forms_keeps_only_the_recognized_names() {
        let forms = parse_forms(Some("a,bogus,img"));

        assert_eq!(forms, vec![Form::Anchor, Form::Image]);
    }

    #[test]
    fn parse_forms_falls_back_to_every_form_when_nothing_is_recognized() {
        assert_eq!(parse_forms(Some("bogus,also-bogus")).len(), Form::ALL.len());
    }
}
