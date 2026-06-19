use crate::fs::global_fs;
use crate::spdx::{SpdxIndex, SpdxLicense, SpdxLicenseDetail};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LicenseInfo {
    pub id: String,
    pub name: String,
    pub is_osi_approved: bool,
    pub is_fsf_libre: bool,
    pub is_deprecated: bool,
}

/// Unified license data provider — owns the SPDX index, template cache
/// interaction, and HTTP fetching. Single source of truth for all license
/// operations.
pub struct LicenseProvider {
    index: SpdxIndex,
    api_cache_dir: PathBuf,
}

impl LicenseProvider {
    /// Create a provider with the default API cache directory.
    pub fn load() -> Result<Self> {
        let index = SpdxIndex::load().context("Failed to load SPDX license index")?;
        let api_cache_dir = dirs::cache_dir()
            .context("unable to determine cache directory")?
            .join("licencify")
            .join("api");
        Ok(Self {
            index,
            api_cache_dir,
        })
    }

    /// Create a provider with a custom API cache directory.
    pub fn with_api_cache(api_cache_dir: &Path) -> Result<Self> {
        let index = SpdxIndex::load().context("Failed to load SPDX license index")?;
        Ok(Self {
            index,
            api_cache_dir: api_cache_dir.to_path_buf(),
        })
    }

    /// Find a license by exact ID.
    #[allow(dead_code)]
    pub fn find(&self, license_id: &str) -> Option<&SpdxLicense> {
        self.index.find(license_id)
    }

    /// Validate a license ID — returns the canonical ID or an error.
    /// Rejects deprecated IDs.
    #[allow(dead_code)]
    pub fn validate(&self, license_id: &str) -> Result<String> {
        match self.index.find(license_id) {
            Some(license) => {
                if license.is_deprecated_license_id {
                    anyhow::bail!("License '{}' is deprecated", license_id);
                }
                Ok(license.license_id.clone())
            }
            None => anyhow::bail!("Unknown license ID: '{}'", license_id),
        }
    }

    /// Get structured license info by ID.
    pub fn info(&self, license_id: &str) -> Result<LicenseInfo> {
        let license = self
            .index
            .find(license_id)
            .context(format!("Unknown license ID: '{}'", license_id))?;
        Ok(LicenseInfo {
            id: license.license_id.clone(),
            name: license.name.clone(),
            is_osi_approved: license.is_osi_approved,
            is_fsf_libre: license.is_fsf_libre,
            is_deprecated: license.is_deprecated_license_id,
        })
    }

    /// Check if a license detail is already cached locally.
    pub fn get_cached(&self, license_id: &str) -> Option<SpdxLicenseDetail> {
        let fs = global_fs();
        let cache_path = self.api_cache_dir.join(format!("{license_id}.json"));
        let text = fs.read_to_string(&cache_path)?;
        serde_json::from_str(&text).ok()
    }

    /// Fetch full license detail from SPDX (with disk caching).
    /// Caller should check `get_cached` first to avoid redundant disk reads.
    pub fn fetch_detail(&self, license_id: &str) -> Result<SpdxLicenseDetail> {
        let fs = global_fs();
        let cache_path = self.api_cache_dir.join(format!("{license_id}.json"));

        let url = format!("https://spdx.org/licenses/{license_id}.json");
        let mut resp = ureq::get(&url)
            .call()
            .context(format!("Failed to fetch license detail for '{license_id}'"))?;
        let body = resp
            .body_mut()
            .read_to_string()
            .context("Failed to read response body")?;

        if let Some(parent) = cache_path.parent() {
            fs.create_dir_all(parent)?;
        }
        fs.write(&cache_path, &body)?;
        Ok(serde_json::from_str(&body)?)
    }

    /// Search licenses by query (matches name or ID).
    pub fn search(&self, query: &str) -> Vec<&SpdxLicense> {
        self.index.search(query)
    }

    /// Return all licenses in the index.
    pub fn all_licenses(&self) -> &[SpdxLicense] {
        &self.index.licenses
    }
}
