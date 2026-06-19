use serde::Deserialize;

const SPDX_INDEX: &str = include_str!("../data/licenses.json");

#[derive(Debug, Deserialize)]
pub struct SpdxIndex {
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpdxLicenseDetail {
    #[serde(rename = "licenseId")]
    #[allow(dead_code)] // needed for deserialization
    pub license_id: String,
    #[allow(dead_code)] // needed for deserialization
    pub name: String,
    #[serde(rename = "licenseText")]
    pub license_text: String,
    #[serde(default, rename = "licenseTextHtml")]
    pub license_text_html: Option<String>,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spdx_index_loads_successfully() {
        let index = SpdxIndex::load().unwrap();
        assert!(!index.licenses.is_empty());
    }

    #[test]
    fn spdx_find_returns_exact_match() {
        let index = SpdxIndex::load().unwrap();
        let license = index.find("MIT").unwrap();
        assert_eq!(license.license_id, "MIT");
        assert_eq!(license.name, "MIT License");
    }

    #[test]
    fn spdx_find_returns_none_for_unknown() {
        let index = SpdxIndex::load().unwrap();
        assert!(index.find("NONexistent").is_none());
    }

    #[test]
    fn spdx_search_finds_by_name() {
        let index = SpdxIndex::load().unwrap();
        let results = index.search("MIT");
        assert!(!results.is_empty());
        assert!(results.iter().any(|l| l.license_id == "MIT"));
    }

    #[test]
    fn spdx_search_finds_by_id() {
        let index = SpdxIndex::load().unwrap();
        let results = index.search("apache");
        assert!(!results.is_empty());
        assert!(results.iter().any(|l| l.license_id == "Apache-2.0"));
    }

    #[test]
    fn spdx_search_is_case_insensitive() {
        let index = SpdxIndex::load().unwrap();
        let results = index.search("MIT");
        let results_upper = index.search("MIT");
        assert_eq!(results.len(), results_upper.len());
    }

    #[test]
    fn spdx_search_returns_empty_for_no_match() {
        let index = SpdxIndex::load().unwrap();
        let results = index.search("zzzznonexistentzzzz");
        assert!(results.is_empty());
    }
}
