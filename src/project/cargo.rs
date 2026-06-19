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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MemFs;

    fn sample_cargo_toml() -> &'static str {
        r#"[package]
name = "test-project"
version = "0.1.0"
license = "MIT"
"#
    }

    #[test]
    fn cargo_exists_returns_true_when_file_present() {
        let fs = MemFs::new();
        fs.write_file(Path::new(CARGO_TOML), sample_cargo_toml());
        let handler = CargoHandler;
        assert!(handler.exists(&fs));
    }

    #[test]
    fn cargo_exists_returns_false_when_file_absent() {
        let fs = MemFs::new();
        let handler = CargoHandler;
        assert!(!handler.exists(&fs));
    }

    #[test]
    fn cargo_update_sets_license_field() {
        let fs = MemFs::new();
        fs.write_file(
            Path::new(CARGO_TOML),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        );
        let handler = CargoHandler;
        handler.update(&fs, "Apache-2.0").unwrap();

        let content = fs.read_to_string(Path::new(CARGO_TOML)).unwrap();
        assert!(content.contains("Apache-2.0"));
    }

    #[test]
    fn cargo_update_preserves_existing_fields() {
        let fs = MemFs::new();
        fs.write_file(Path::new(CARGO_TOML), sample_cargo_toml());
        let handler = CargoHandler;
        handler.update(&fs, "GPL-3.0-only").unwrap();

        let content = fs.read_to_string(Path::new(CARGO_TOML)).unwrap();
        assert!(content.contains("test-project"));
        assert!(content.contains("GPL-3.0-only"));
    }

    #[test]
    fn cargo_name_returns_correct_manifest_name() {
        let handler = CargoHandler;
        assert_eq!(handler.name(), "Cargo.toml");
    }
}
