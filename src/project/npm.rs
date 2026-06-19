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
        pkg["license"] = serde_json::Value::String(license_id.to_string());
        let formatted = serde_json::to_string_pretty(&pkg)
            .with_context(|| "failed to serialize package.json")?;
        let output = format!("{formatted}\n");
        fs.write(Path::new(PACKAGE_JSON), &output)
            .with_context(|| format!("failed to write {PACKAGE_JSON}"))?;
        Ok(())
    }
}
