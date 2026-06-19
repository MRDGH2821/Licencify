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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_replaces_year_placeholder() {
        let template = "Copyright {{ year }} Author";
        let result = render(template, "2024", "Alice").unwrap();
        assert_eq!(result, "Copyright 2024 Author");
    }

    #[test]
    fn render_replaces_author_placeholder() {
        let template = "Copyright 2024 {{ author }}";
        let result = render(template, "2024", "Alice").unwrap();
        assert_eq!(result, "Copyright 2024 Alice");
    }

    #[test]
    fn render_replaces_spdx_year_placeholder() {
        let template = "Copyright <year> Author";
        let result = render(template, "2024", "Alice").unwrap();
        assert_eq!(result, "Copyright 2024 Author");
    }

    #[test]
    fn render_replaces_spdx_author_placeholder() {
        let template = "Copyright 2024 <author>";
        let result = render(template, "2024", "Alice").unwrap();
        assert_eq!(result, "Copyright 2024 Alice");
    }

    #[test]
    fn render_replaces_copyright_holders_placeholder() {
        let template = "Copyright <copyright holders>";
        let result = render(template, "2024", "Alice").unwrap();
        assert_eq!(result, "Copyright Alice");
    }

    #[test]
    fn render_handles_html_encoded_placeholders() {
        let template = "Copyright &lt;year&gt; &lt;author&gt;";
        let result = render(template, "2024", "Alice").unwrap();
        assert_eq!(result, "Copyright 2024 Alice");
    }

    #[test]
    fn render_preserves_literal_text() {
        let template = "MIT License\n\nPermission is hereby granted...";
        let result = render(template, "2024", "Alice").unwrap();
        assert_eq!(result, "MIT License\n\nPermission is hereby granted...");
    }

    #[test]
    fn default_year_returns_current_year() {
        let year = default_year();
        assert_eq!(year.len(), 4);
        assert!(year.chars().all(|c| c.is_ascii_digit()));
    }
}
