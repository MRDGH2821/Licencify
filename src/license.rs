use crate::spdx::SpdxIndex;
use anyhow::{Context, Result};

/// Validate a license ID against the SPDX index
pub fn validate_license_id(license_id: &str) -> Result<String> {
    let index = SpdxIndex::load().context("Failed to load SPDX license index")?;

    match index.find(license_id) {
        Some(license) => {
            if license.is_deprecated_license_id {
                anyhow::bail!("License '{}' is deprecated", license_id);
            }
            Ok(license.license_id.clone())
        }
        None => anyhow::bail!("Unknown license ID: '{}'", license_id),
    }
}

/// Check if a license ID is valid (without error)
pub fn is_valid_license_id(license_id: &str) -> bool {
    if let Ok(index) = SpdxIndex::load() {
        index.find(license_id).is_some()
    } else {
        false
    }
}

/// Get license info by ID
pub fn get_license_info(license_id: &str) -> Result<LicenseInfo> {
    let index = SpdxIndex::load().context("Failed to load SPDX license index")?;

    let license = index
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

/// License information
#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub id: String,
    pub name: String,
    pub is_osi_approved: bool,
    pub is_fsf_libre: bool,
    pub is_deprecated: bool,
}
