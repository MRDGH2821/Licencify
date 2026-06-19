use crate::fs::global_fs;
use anyhow::Result;

mod cargo;
pub mod handler;
mod npm;
mod python;

/// Update the license field in every manifest found in the current directory.
///
/// Returns a list of manifests that were updated (e.g. `["Cargo.toml", "package.json"]`).
pub fn update_manifest(license_id: &str, _author: &str, _year: &str) -> Result<Vec<String>> {
    let fs = global_fs();
    let mut updated = Vec::new();
    for handler in handlers() {
        if handler.exists(&*fs) {
            handler.update(&*fs, license_id)?;
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
