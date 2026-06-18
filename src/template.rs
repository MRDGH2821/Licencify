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

/// Resolve the default author from git config.
pub fn default_author() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .context("Failed to run git config")?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Ok(name);
        }
    }

    Err(anyhow::anyhow!(
        "Could not determine author name. Use --author to specify it."
    ))
}

/// Resolve the default year (current year).
pub fn default_year() -> String {
    chrono::Local::now().format("%Y").to_string()
}
