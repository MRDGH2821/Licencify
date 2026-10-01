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
        let notice = if super::is_proprietary_selection(license_id) {
            Some(super::require_primary_notice(fs)?)
        } else {
            None
        };
        let package = doc
            .get_mut("package")
            .and_then(|item| item.as_table_like_mut())
            .with_context(|| format!("{CARGO_TOML} has no [package] table"))?;
        if let Some(notice) = notice {
            package.remove("license");
            package.insert("license-file", toml_edit::value(notice));
            package.insert("publish", toml_edit::value(false));
        } else {
            if package.get("publish").and_then(|item| item.as_bool()) == Some(false) {
                eprintln!(
                    "Warning: Cargo.toml publish = false remains set after switching to {license_id}. Change that flag before publishing."
                );
            }
            package.remove("license-file");
            package.insert("license", toml_edit::value(license_id));
        }
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

    #[test]
    fn cargo_proprietary_sets_private_notice_without_spdx_expression() {
        let fs = MemFs::new();
        fs.write_file(Path::new(CARGO_TOML), sample_cargo_toml());
        fs.write_file(Path::new("LICENCE.txt"), "All Rights Reserved\n");
        CargoHandler.update(&fs, "proprietary").unwrap();

        let content = fs.read_to_string(Path::new(CARGO_TOML)).unwrap();
        assert!(content.contains("name = \"test-project\""), "{content}");
        assert!(content.contains("publish = false"), "{content}");
        assert!(
            content.contains("license-file = \"LICENCE.txt\""),
            "{content}"
        );
        assert!(!content.contains("license ="), "{content}");
        assert!(!content.contains("UNLICENSED"), "{content}");
        assert!(!content.contains("proprietary"), "{content}");
    }

    #[test]
    fn cargo_proprietary_without_notice_does_not_write() {
        let fs = MemFs::new();
        fs.write_file(Path::new(CARGO_TOML), sample_cargo_toml());
        let error = CargoHandler.update(&fs, "proprietary").unwrap_err();
        assert!(
            error.to_string().contains("existing primary notice"),
            "{error}"
        );
        let content = fs.read_to_string(Path::new(CARGO_TOML)).unwrap();
        assert!(content.contains("license = \"MIT\""), "{content}");
        assert!(!content.contains("license-file"), "{content}");
        assert!(!content.contains("publish"), "{content}");
    }

    #[test]
    fn cargo_spdx_switch_keeps_publish_false_and_drops_license_file() {
        let fs = MemFs::new();
        fs.write_file(
            Path::new(CARGO_TOML),
            "[package]\nname = \"test-project\"\nversion = \"0.1.0\"\npublish = false\nlicense-file = \"LICENCE.txt\"\n",
        );
        CargoHandler.update(&fs, "Apache-2.0").unwrap();

        let content = fs.read_to_string(Path::new(CARGO_TOML)).unwrap();
        assert!(content.contains("publish = false"), "{content}");
        assert!(content.contains("license = \"Apache-2.0\""), "{content}");
        assert!(!content.contains("license-file"), "{content}");
    }
}
