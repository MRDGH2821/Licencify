use super::handler::ManifestHandler;
use crate::fs::Fs;
use anyhow::{Context, Result};
use std::path::Path;

const PACKAGE_JSON: &str = "package.json";

pub struct NpmHandler;

impl ManifestHandler for NpmHandler {
    fn name(&self) -> &str {
        PACKAGE_JSON
    }

    fn exists(&self, fs: &dyn Fs) -> bool {
        fs.exists(Path::new(PACKAGE_JSON))
    }

    fn update(&self, fs: &dyn Fs, license_id: &str) -> Result<()> {
        let content = fs
            .read_to_string(Path::new(PACKAGE_JSON))
            .with_context(|| format!("{PACKAGE_JSON} is not valid JSON"))?;
        let mut pkg: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("{PACKAGE_JSON} is not valid JSON"))?;
        if super::is_proprietary_selection(license_id) {
            let notice = super::require_primary_notice(fs)?;
            pkg["private"] = serde_json::Value::Bool(true);
            pkg["license"] = serde_json::Value::String(format!("SEE LICENSE IN {notice}"));
        } else {
            if pkg.get("private").and_then(|value| value.as_bool()) == Some(true) {
                eprintln!(
                    "Warning: package.json private = true remains set after switching to {license_id}. Change that flag before publishing."
                );
            }
            pkg["license"] = serde_json::Value::String(license_id.to_string());
        }
        let formatted = serde_json::to_string_pretty(&pkg)
            .with_context(|| "failed to serialize package.json")?;
        let output = format!("{formatted}\n");
        fs.write(Path::new(PACKAGE_JSON), &output)
            .with_context(|| format!("failed to write {PACKAGE_JSON}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MemFs;

    #[test]
    fn npm_proprietary_sets_private_and_see_license_pointer() {
        let fs = MemFs::new();
        fs.write_file(
            Path::new(PACKAGE_JSON),
            "{\n  \"name\": \"demo\",\n  \"license\": \"MIT\"\n}\n",
        );
        fs.write_file(Path::new("LICENSE.md"), "notice\n");
        NpmHandler.update(&fs, "proprietary").unwrap();

        let content = fs.read_to_string(Path::new(PACKAGE_JSON)).unwrap();
        assert!(content.contains("\"private\": true"), "{content}");
        assert!(content.contains("SEE LICENSE IN LICENSE.md"), "{content}");
        assert!(!content.contains("UNLICENSED"), "{content}");
        assert!(!content.contains("proprietary"), "{content}");
    }

    #[test]
    fn npm_proprietary_without_notice_does_not_write() {
        let fs = MemFs::new();
        fs.write_file(
            Path::new(PACKAGE_JSON),
            "{\n  \"name\": \"demo\",\n  \"license\": \"MIT\"\n}\n",
        );
        let error = NpmHandler.update(&fs, "UNLICENSED").unwrap_err();
        assert!(
            error.to_string().contains("existing primary notice"),
            "{error}"
        );
        let content = fs.read_to_string(Path::new(PACKAGE_JSON)).unwrap();
        assert!(content.contains("\"license\": \"MIT\""), "{content}");
        assert!(!content.contains("private"), "{content}");
    }

    #[test]
    fn npm_spdx_switch_keeps_private_true() {
        let fs = MemFs::new();
        fs.write_file(
            Path::new(PACKAGE_JSON),
            "{\n  \"name\": \"demo\",\n  \"private\": true,\n  \"license\": \"SEE LICENSE IN LICENCE.txt\"\n}\n",
        );
        NpmHandler.update(&fs, "MIT").unwrap();

        let content = fs.read_to_string(Path::new(PACKAGE_JSON)).unwrap();
        assert!(content.contains("\"private\": true"), "{content}");
        assert!(content.contains("\"license\": \"MIT\""), "{content}");
        assert!(!content.contains("SEE LICENSE IN"), "{content}");
    }
}
