use anyhow::{Context, Result};
use std::fs;
use toml_edit::DocumentMut;

const CARGO_TOML: &str = "Cargo.toml";

/// Returns `true` if `Cargo.toml` exists in the current directory.
pub fn exists() -> bool {
    fs::metadata(CARGO_TOML).is_ok()
}

/// Read `Cargo.toml` and set the `[package].license` field to `license_id`.
pub fn update(license_id: &str) -> Result<()> {
    let content =
        fs::read_to_string(CARGO_TOML).with_context(|| format!("failed to read {CARGO_TOML}"))?;

    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("failed to parse {CARGO_TOML} as TOML"))?;

    let package = doc
        .get_mut("package")
        .and_then(|item| item.as_table_like_mut())
        .with_context(|| format!("{CARGO_TOML} has no [package] table"))?;

    package.insert("license", toml_edit::value(license_id));

    fs::write(CARGO_TOML, doc.to_string())
        .with_context(|| format!("failed to write {CARGO_TOML}"))?;

    Ok(())
}
