use crate::spdx::{SpdxIndex, SpdxLicenseDetail};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Registry {
    index: SpdxIndex,
    cache_dir: PathBuf,
}

impl Registry {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        let index = SpdxIndex::load().context("Failed to load SPDX license index")?;
        Ok(Self {
            index,
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Get detail URL for a license
    pub fn detail_url(&self, license_id: &str) -> Option<String> {
        self.index
            .find(license_id)
            .and_then(|l| l.details_url.clone())
    }

    /// Fetch license detail from SPDX (with caching)
    pub fn fetch_detail(&self, license_id: &str) -> Result<SpdxLicenseDetail> {
        let cache_path = self.cache_dir.join(format!("{license_id}.json"));

        // Check cache first
        if cache_path.exists() {
            let text = fs::read_to_string(&cache_path)?;
            return Ok(serde_json::from_str(&text)?);
        }

        // Build URL (SPDX convention)
        let url = format!("https://spdx.org/licenses/{license_id}.json");

        // Fetch from SPDX
        let mut resp = ureq::get(&url)
            .call()
            .context(format!("Failed to fetch license detail for '{license_id}'"))?;

        let body = resp
            .body_mut()
            .read_to_string()
            .context("Failed to read response body")?;

        // Cache it
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&cache_path, &body)?;

        Ok(serde_json::from_str(&body)?)
    }

    /// Search licenses by query
    pub fn search(&self, query: &str) -> Vec<&crate::spdx::SpdxLicense> {
        self.index.search(query)
    }

    /// Get all licenses
    pub fn all_licenses(&self) -> &[crate::spdx::SpdxLicense] {
        &self.index.licenses
    }

    /// Find license by ID
    pub fn find(&self, license_id: &str) -> Option<&crate::spdx::SpdxLicense> {
        self.index.find(license_id)
    }
}
