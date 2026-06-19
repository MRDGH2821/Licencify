use anyhow::{Context, Result};
use regex::Regex;
use std::sync::LazyLock;
use tera::{Context as TeraContext, Tera};

static RE_YEAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<year>|&lt;year&gt;").unwrap());
static RE_AUTHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<author>|&lt;author&gt;").unwrap());
static RE_HOLDERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<copyright holders?>|&lt;copyright holders?&gt;").unwrap());

/// Render a license template with full context (company, email, date).
pub fn render_with_context(template: &str, ctx: &RenderContext) -> Result<String> {
    // Step 1: Try Tera rendering for {{ year }}, {{ author }}, {{ company }}, etc.
    let mut tera_ctx = TeraContext::new();
    tera_ctx.insert("year", &ctx.year);
    tera_ctx.insert("author", &ctx.author);
    tera_ctx.insert("company", &ctx.company);
    tera_ctx.insert("email", &ctx.email);
    tera_ctx.insert("date", &ctx.date);
    let rendered =
        Tera::one_off(template, &tera_ctx, false).context("Failed to render license template")?;

    // Step 2: Post-replace SPDX-style <year>, <author>, <copyright holders> placeholders
    let result = replace_spdx_placeholders(&rendered, &ctx.year, &ctx.author);
    Ok(result)
}

/// Full render context for template placeholders.
pub struct RenderContext {
    pub year: String,
    pub author: String,
    pub company: String,
    pub email: String,
    pub date: String,
}

/// Build a RenderContext with sensible defaults.
pub fn render_context(
    year: &str,
    author: &str,
    company: Option<&str>,
    email: Option<&str>,
) -> RenderContext {
    RenderContext {
        year: year.to_string(),
        author: author.to_string(),
        company: company.unwrap_or(author).to_string(),
        email: email.unwrap_or("").to_string(),
        date: chrono::Local::now().format("%Y-%m-%d").to_string(),
    }
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

    fn ctx(year: &str, author: &str) -> RenderContext {
        render_context(year, author, None, None)
    }

    #[test]
    fn render_replaces_year_placeholder() {
        let template = "Copyright {{ year }} Author";
        let result = render_with_context(template, &ctx("2024", "Alice")).unwrap();
        assert_eq!(result, "Copyright 2024 Author");
    }

    #[test]
    fn render_replaces_author_placeholder() {
        let template = "Copyright 2024 {{ author }}";
        let result = render_with_context(template, &ctx("2024", "Alice")).unwrap();
        assert_eq!(result, "Copyright 2024 Alice");
    }

    #[test]
    fn render_replaces_spdx_year_placeholder() {
        let template = "Copyright <year> Author";
        let result = render_with_context(template, &ctx("2024", "Alice")).unwrap();
        assert_eq!(result, "Copyright 2024 Author");
    }

    #[test]
    fn render_replaces_spdx_author_placeholder() {
        let template = "Copyright 2024 <author>";
        let result = render_with_context(template, &ctx("2024", "Alice")).unwrap();
        assert_eq!(result, "Copyright 2024 Alice");
    }

    #[test]
    fn render_replaces_copyright_holders_placeholder() {
        let template = "Copyright <copyright holders>";
        let result = render_with_context(template, &ctx("2024", "Alice")).unwrap();
        assert_eq!(result, "Copyright Alice");
    }

    #[test]
    fn render_handles_html_encoded_placeholders() {
        let template = "Copyright &lt;year&gt; &lt;author&gt;";
        let result = render_with_context(template, &ctx("2024", "Alice")).unwrap();
        assert_eq!(result, "Copyright 2024 Alice");
    }

    #[test]
    fn render_preserves_literal_text() {
        let template = "MIT License\n\nPermission is hereby granted...";
        let result = render_with_context(template, &ctx("2024", "Alice")).unwrap();
        assert_eq!(result, "MIT License\n\nPermission is hereby granted...");
    }

    #[test]
    fn default_year_returns_current_year() {
        let year = default_year();
        assert_eq!(year.len(), 4);
        assert!(year.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn render_with_context_replaces_company() {
        let template = "Copyright (C) {{ company }} - All Rights Reserved";
        let ctx = render_context("2024", "Alice", Some("Acme Corp"), Some("alice@acme.com"));
        let result = render_with_context(template, &ctx).unwrap();
        assert_eq!(result, "Copyright (C) Acme Corp - All Rights Reserved");
    }

    #[test]
    fn render_with_context_replaces_email() {
        let template = "Written by {{ author }} <{{ email }}>";
        let ctx = render_context("2024", "Alice", None, Some("alice@acme.com"));
        let result = render_with_context(template, &ctx).unwrap();
        assert_eq!(result, "Written by Alice <alice@acme.com>");
    }

    #[test]
    fn render_with_context_replaces_date() {
        let template = "Date: {{ date }}";
        let ctx = render_context("2024", "Alice", None, None);
        let result = render_with_context(template, &ctx).unwrap();
        assert!(result.starts_with("Date: 20"));
        assert_eq!(result.len(), "Date: 2024-01-01".len()); // YYYY-MM-DD
    }

    #[test]
    fn render_with_context_company_defaults_to_author() {
        let template = "Copyright {{ company }}";
        let ctx = render_context("2024", "Alice", None, None);
        let result = render_with_context(template, &ctx).unwrap();
        assert_eq!(result, "Copyright Alice");
    }

    #[test]
    fn render_proprietary_template_full() {
        let tmpl = include_str!("../templates/licence/proprietary.tera");
        let ctx = render_context("2024", "Alice", Some("Acme Corp"), Some("alice@acme.com"));
        let result = render_with_context(tmpl, &ctx).unwrap();
        assert!(result.contains("Acme Corp"));
        assert!(result.contains("Alice"));
        assert!(result.contains("alice@acme.com"));
        assert!(result.contains("All Rights Reserved"));
        assert!(result.contains("Proprietary and confidential"));
    }
}
