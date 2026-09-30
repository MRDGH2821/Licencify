use crate::fs::global_fs;
use crate::spdx::{SpdxIndex, SpdxLicense, SpdxLicenseDetail};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub id: String,
    pub name: String,
}

/// Unified license data provider — owns the SPDX index, template cache
/// interaction, and HTTP fetching. Single source of truth for all license
/// operations.
pub struct LicenseProvider {
    index: SpdxIndex,
    cache_dir: PathBuf,
}

/// Return the shared user-level SPDX detail cache directory.
pub fn spdx_cache_dir() -> Result<PathBuf> {
    Ok(dirs::cache_dir()
        .context("unable to determine cache directory")?
        .join("licencify")
        .join("SPDX-Cache"))
}

fn validate_spdx_id(license_id: &str) -> Result<()> {
    anyhow::ensure!(
        !license_id.is_empty()
            && license_id != "."
            && license_id != ".."
            && license_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.'),
        "Invalid SPDX license identifier: '{license_id}'"
    );
    Ok(())
}

fn parse_and_cache_detail(
    fs: &dyn crate::fs::Fs,
    cache_path: &Path,
    body: &str,
) -> Result<SpdxLicenseDetail> {
    let detail: SpdxLicenseDetail =
        serde_json::from_str(body).context("Failed to parse SPDX license detail response")?;
    let write_result = cache_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("SPDX cache path has no parent"))
        .and_then(|parent| {
            fs.create_dir_all(parent)
                .context("Failed to create SPDX cache")
        })
        .and_then(|()| {
            fs.write(cache_path, body)
                .context("Failed to write SPDX detail cache")
        });
    if let Err(error) = write_result {
        eprintln!(
            "Warning: fetched {}, but could not cache SPDX detail: {error:#}",
            detail.license_id
        );
    }
    Ok(detail)
}

impl LicenseProvider {
    /// Create a provider with the default SPDX detail cache directory.
    pub fn load() -> Result<Self> {
        let index = SpdxIndex::load().context("Failed to load SPDX license index")?;
        Ok(Self {
            index,
            cache_dir: spdx_cache_dir()?,
        })
    }

    /// Create a provider with a custom cache directory for tests and callers.
    pub fn with_spdx_cache(cache_dir: &Path) -> Result<Self> {
        let index = SpdxIndex::load().context("Failed to load SPDX license index")?;
        Ok(Self {
            index,
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Get structured license info by ID.
    pub fn info(&self, license_id: &str) -> Result<LicenseInfo> {
        // "proprietary" is not a real SPDX ID — short-circuit
        if license_id.eq_ignore_ascii_case("proprietary") {
            return Ok(LicenseInfo {
                id: "UNLICENSED".to_string(),
                name: "Proprietary (No Licence)".to_string(),
            });
        }
        let license = self
            .index
            .find(license_id)
            .context(format!("Unknown license ID: '{}'", license_id))?;
        Ok(LicenseInfo {
            id: license.license_id.clone(),
            name: license.name.clone(),
        })
    }

    /// Check if a license detail is already cached locally.
    pub fn get_cached(&self, license_id: &str) -> Option<SpdxLicenseDetail> {
        validate_spdx_id(license_id).ok()?;
        let fs = global_fs();
        let cache_path = self.cache_dir.join(format!("{license_id}.json"));
        let text = fs.read_to_string(&cache_path)?;
        serde_json::from_str(&text).ok()
    }

    /// Fetch full license detail from SPDX (with disk caching).
    /// Caller should check `get_cached` first to avoid redundant disk reads.
    pub fn fetch_detail(&self, license_id: &str) -> Result<SpdxLicenseDetail> {
        self.index
            .find(license_id)
            .context(format!("Unknown license ID: '{license_id}'"))?;
        let fs = global_fs();
        let cache_path = self.cache_dir.join(format!("{license_id}.json"));

        let url = format!("https://spdx.org/licenses/{license_id}.json");
        let mut resp = ureq::get(&url)
            .call()
            .context(format!("Failed to fetch license detail for '{license_id}'"))?;
        let body = resp
            .body_mut()
            .read_to_string()
            .context("Failed to read response body")?;
        parse_and_cache_detail(fs.as_ref(), &cache_path, &body)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_uses_shared_spdx_directory() {
        assert!(
            spdx_cache_dir()
                .unwrap()
                .ends_with(Path::new("licencify").join("SPDX-Cache"))
        );
    }

    struct FailingCacheFs;

    impl crate::fs::Fs for FailingCacheFs {
        fn read_to_string(&self, _: &Path) -> Option<String> {
            None
        }

        fn write(&self, _: &Path, _: &str) -> std::io::Result<()> {
            Err(std::io::Error::other("cache unavailable"))
        }

        fn exists(&self, _: &Path) -> bool {
            false
        }

        fn create_dir_all(&self, _: &Path) -> std::io::Result<()> {
            Err(std::io::Error::other("cache unavailable"))
        }

        fn read_dir(&self, _: &Path) -> Vec<PathBuf> {
            Vec::new()
        }

        fn remove_dir_all(&self, _: &Path) -> std::io::Result<()> {
            Err(std::io::Error::other("cache unavailable"))
        }
    }

    #[test]
    fn cache_write_failure_keeps_successfully_parsed_detail() {
        let detail = parse_and_cache_detail(
            &FailingCacheFs,
            Path::new("/cache/MIT.json"),
            r#"{"licenseId":"MIT","name":"MIT License","licenseText":"successful response"}"#,
        )
        .unwrap();

        assert_eq!(detail.license_text, "successful response");
    }
}
