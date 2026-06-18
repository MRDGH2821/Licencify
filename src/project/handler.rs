use anyhow::Result;

/// Trait for project manifest handlers (Cargo.toml, package.json, etc.).
pub trait ManifestHandler {
    /// Human-readable name of this manifest (e.g. "Cargo.toml").
    fn name(&self) -> &str;

    /// Returns true if this manifest exists in the current directory.
    fn exists(&self) -> bool;

    /// Update the license field in this manifest.
    fn update(&self, license_id: &str) -> Result<()>;
}
