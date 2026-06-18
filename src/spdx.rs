use serde::Deserialize;
use std::collections::HashMap;

const SPDX_INDEX: &str = include_str!("../data/licenses.json");

#[derive(Debug, Deserialize)]
pub struct SpdxIndex {
    #[serde(rename = "licenseListVersion")]
    pub license_list_version: String,
    pub licenses: Vec<SpdxLicense>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpdxLicense {
    #[serde(rename = "licenseId")]
    pub license_id: String,
    pub name: String,
    #[serde(default, rename = "isDeprecatedLicenseId")]
    pub is_deprecated_license_id: bool,
    #[serde(default, rename = "isOsiApproved")]
    pub is_osi_approved: bool,
    #[serde(default, rename = "isFsfLibre")]
    pub is_fsf_libre: bool,
    #[serde(rename = "detailsUrl")]
    pub details_url: Option<String>,
    #[serde(default, rename = "seeAlso")]
    pub see_also: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpdxLicenseDetail {
    #[serde(rename = "licenseId")]
    pub license_id: String,
    pub name: String,
    #[serde(rename = "licenseText")]
    pub license_text: String,
    #[serde(default, rename = "isOsiApproved")]
    pub is_osi_approved: bool,
    #[serde(default, rename = "isFsfLibre")]
    pub is_fsf_libre: bool,
}

impl SpdxIndex {
    pub fn load() -> anyhow::Result<Self> {
        let index: SpdxIndex = serde_json::from_str(SPDX_INDEX)?;
        Ok(index)
    }

    pub fn find(&self, license_id: &str) -> Option<&SpdxLicense> {
        self.licenses.iter().find(|l| l.license_id == license_id)
    }

    pub fn search(&self, query: &str) -> Vec<&SpdxLicense> {
        let q = query.to_lowercase();
        self.licenses
            .iter()
            .filter(|l| {
                l.name.to_lowercase().contains(&q) || l.license_id.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn by_id(&self) -> HashMap<&str, &SpdxLicense> {
        self.licenses
            .iter()
            .map(|l| (l.license_id.as_str(), l))
            .collect()
    }
}
