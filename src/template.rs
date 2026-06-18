use anyhow::{Context, Result};

/// Render a license template by replacing placeholders.
///
/// Supported placeholders:
/// - `[year]` / `<year>` → provided year
/// - `[fullname]` / `<copyright holders>` → provided author
pub fn render(template: &str, year: &str, author: &str) -> String {
    template
        .replace("[year]", year)
        .replace("<year>", year)
        .replace("[fullname]", author)
        .replace("<copyright holders>", author)
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
