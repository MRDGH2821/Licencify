use anyhow::{Context, Result};
use tera::{Context as TeraContext, Tera};

/// Render a license template using Tera.
///
/// Supported template variables:
/// - `{{ year }}` — the copyright year
/// - `{{ author }}` — the copyright holder name
pub fn render(template: &str, year: &str, author: &str) -> Result<String> {
    let mut ctx = TeraContext::new();
    ctx.insert("year", year);
    ctx.insert("author", author);
    Tera::one_off(template, &ctx, false).context("Failed to render license template")
}

/// Resolve the default year (current year).
pub fn default_year() -> String {
    chrono::Local::now().format("%Y").to_string()
}
