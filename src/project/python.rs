use super::handler::ManifestHandler;
use anyhow::{Context, Result};
use std::fs;
use toml_edit::DocumentMut;

const PYPROJECT_TOML: &str = "pyproject.toml";

pub struct PythonHandler;

impl ManifestHandler for PythonHandler {
    fn name(&self) -> &str {
        PYPROJECT_TOML
    }

    fn exists(&self) -> bool {
        fs::metadata(PYPROJECT_TOML).is_ok()
    }

    fn update(&self, license_id: &str) -> Result<()> {
        let content = fs::read_to_string(PYPROJECT_TOML)
            .with_context(|| format!("failed to read {PYPROJECT_TOML}"))?;
        let mut doc: DocumentMut = content
            .parse()
            .with_context(|| format!("failed to parse {PYPROJECT_TOML} as TOML"))?;
        let project = doc
            .get_mut("project")
            .and_then(|item| item.as_table_like_mut())
            .with_context(|| format!("{PYPROJECT_TOML} has no [project] table"))?;
        // PEP 639-style `license = { text = "MIT" }`
        let mut table = toml_edit::InlineTable::new();
        table.insert("text", toml_edit::Value::from(license_id));
        project.insert("license", toml_edit::value(table));
        fs::write(PYPROJECT_TOML, doc.to_string())
            .with_context(|| format!("failed to write {PYPROJECT_TOML}"))?;
        Ok(())
    }
}
