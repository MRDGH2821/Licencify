use super::handler::ManifestHandler;
use anyhow::{Context, Result};
use std::fs;

const PACKAGE_JSON: &str = "package.json";

pub struct NpmHandler;

impl ManifestHandler for NpmHandler {
    fn name(&self) -> &str {
        PACKAGE_JSON
    }

    fn exists(&self) -> bool {
        fs::metadata(PACKAGE_JSON).is_ok()
    }

    fn update(&self, license_id: &str) -> Result<()> {
        let content = fs::read_to_string(PACKAGE_JSON)
            .with_context(|| format!("failed to read {PACKAGE_JSON}"))?;
        let mut pkg: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("{PACKAGE_JSON} is not valid JSON"))?;
        pkg["license"] = serde_json::Value::String(license_id.to_string());
        let formatted = serde_json::to_string_pretty(&pkg)
            .with_context(|| "failed to serialize package.json")?;
        let output = format!("{formatted}\n");
        fs::write(PACKAGE_JSON, output)
            .with_context(|| format!("failed to write {PACKAGE_JSON}"))?;
        Ok(())
    }
}
