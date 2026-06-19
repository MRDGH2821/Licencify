use super::handler::ManifestHandler;
use crate::fs::Fs;
use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::DocumentMut;

const CARGO_TOML: &str = "Cargo.toml";

pub struct CargoHandler;

impl ManifestHandler for CargoHandler {
    fn name(&self) -> &str {
        CARGO_TOML
    }

    fn exists(&self, fs: &dyn Fs) -> bool {
        fs.exists(Path::new(CARGO_TOML))
    }

    fn update(&self, fs: &dyn Fs, license_id: &str) -> Result<()> {
        let content = fs
            .read_to_string(Path::new(CARGO_TOML))
            .with_context(|| format!("failed to read {CARGO_TOML}"))?;
        let mut doc: DocumentMut = content
            .parse()
            .with_context(|| format!("failed to parse {CARGO_TOML} as TOML"))?;
        let package = doc
            .get_mut("package")
            .and_then(|item| item.as_table_like_mut())
            .with_context(|| format!("{CARGO_TOML} has no [package] table"))?;
        package.insert("license", toml_edit::value(license_id));
        fs.write(Path::new(CARGO_TOML), &doc.to_string())
            .with_context(|| format!("failed to write {CARGO_TOML}"))?;
        Ok(())
    }
}
