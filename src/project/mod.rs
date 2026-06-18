use anyhow::Result;

mod cargo;
mod npm;
mod python;

/// Check if any supported project manifest exists in the current directory.
pub fn exists_any() -> bool {
    cargo::exists() || npm::exists() || python::exists()
}

/// Update the license field in every manifest found in the current directory.
///
/// Returns a list of manifests that were updated (e.g. `["Cargo.toml", "package.json"]`).
pub fn update_manifest(license_id: &str, _author: &str, _year: &str) -> Result<Vec<String>> {
    let mut updated = Vec::new();

    if cargo::exists() {
        cargo::update(license_id)?;
        updated.push("Cargo.toml".to_string());
    }

    if npm::exists() {
        npm::update(license_id)?;
        updated.push("package.json".to_string());
    }

    if python::exists() {
        python::update(license_id)?;
        updated.push("pyproject.toml".to_string());
    }

    Ok(updated)
}
