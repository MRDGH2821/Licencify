use crate::fs::global_fs;
use anyhow::Result;

mod cargo;
pub mod handler;
mod npm;
mod python;

/// Normalise SPDX ID for manifest fields.
/// Maps `proprietary` → `UNLICENSED` (the standard SPDX identifier for non-open-source).
fn normalize_manifest_spdx(spdx_id: &str) -> &str {
    if spdx_id.eq_ignore_ascii_case("proprietary") || spdx_id.eq_ignore_ascii_case("UNLICENSED") {
        "UNLICENSED"
    } else {
        spdx_id
    }
}

/// Update the license field in every manifest found in the current directory.
///
/// Returns a list of manifests that were updated (e.g. `["Cargo.toml", "package.json"]`).
pub fn update_manifest(license_id: &str, _author: &str, _year: &str) -> Result<Vec<String>> {
    let normalized = normalize_manifest_spdx(license_id);
    let fs = global_fs();
    let mut updated = Vec::new();
    for handler in handlers() {
        if handler.exists(&*fs) {
            handler.update(&*fs, normalized)?;
            updated.push(handler.name().to_string());
        }
    }
    Ok(updated)
}

fn handlers() -> Vec<Box<dyn handler::ManifestHandler>> {
    vec![
        Box::new(cargo::CargoHandler),
        Box::new(npm::NpmHandler),
        Box::new(python::PythonHandler),
    ]
}
