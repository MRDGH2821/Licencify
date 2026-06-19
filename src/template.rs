use anyhow::{Context, Result};
use regex::Regex;
use std::sync::LazyLock;
use tera::{Context as TeraContext, Tera};

static RE_YEAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<year>|&lt;year&gt;").unwrap());
static RE_AUTHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<author>|&lt;author&gt;").unwrap());
static RE_HOLDERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<copyright holders?>|&lt;copyright holders?&gt;").unwrap());

/// Render a license template, supporting both Tera (`{{ year }}`) and
/// SPDX (`<year>`, `<copyright holders>`) placeholder notations.
pub fn render(template: &str, year: &str, author: &str) -> Result<String> {
    // Step 1: Try Tera rendering for {{ year }}, {{ author }} etc.
    let mut ctx = TeraContext::new();
    ctx.insert("year", year);
    ctx.insert("author", author);
    let rendered =
        Tera::one_off(template, &ctx, false).context("Failed to render license template")?;

    // Step 2: Post-replace SPDX-style <year>, <author>, <copyright holders> placeholders
    let result = replace_spdx_placeholders(&rendered, year, author);
    Ok(result)
}

/// Replace SPDX-style angle-bracket placeholders in licence text.
/// Handles both raw (`<year>`) and HTML-encoded (`&lt;year&gt;`) forms.
fn replace_spdx_placeholders(text: &str, year: &str, author: &str) -> String {
    let text = RE_YEAR.replace_all(text, year);
    let text = RE_AUTHOR.replace_all(&text, author);
    let text = RE_HOLDERS.replace_all(&text, author);
    text.into_owned()
}

/// Resolve the default year (current year).
pub fn default_year() -> String {
    chrono::Local::now().format("%Y").to_string()
}
