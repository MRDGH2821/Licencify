use std::path::PathBuf;

/// The base name for licence files, respecting locale conventions.
///
/// Resolves through: config override → locale detection → default `LICENCE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenceName {
    Licence,
    License,
}

impl LicenceName {
    /// Detect from the system locale (LC_ALL, LANG, etc.).
    pub fn detect() -> Self {
        for var in &["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
            if let Ok(val) = std::env::var(var) {
                let lower = val.to_lowercase();
                if lower.contains("en_gb")
                    || lower.contains("en-gb")
                    || lower.contains("en.au")
                    || lower.contains("en_nz")
                    || lower.contains("en-in")
                {
                    return Self::Licence;
                }
                if lower.starts_with("en") {
                    return Self::License;
                }
            }
        }
        Self::Licence
    }

    /// Resolve from config override, falling back to locale detection.
    pub fn resolve(config_licence_name: Option<&str>) -> Self {
        match config_licence_name {
            Some(name) if name.eq_ignore_ascii_case("LICENSE") => Self::License,
            Some(_) => Self::Licence,
            None => Self::detect(),
        }
    }

    /// The raw string form (e.g. `"LICENCE"` or `"LICENSE"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Licence => "LICENCE",
            Self::License => "LICENSE",
        }
    }

    /// Build the licence filename with the given extension.
    pub fn file_path(&self, ext: &str) -> PathBuf {
        PathBuf::from(format!("{}.{}", self.as_str(), ext))
    }

    /// Primary filenames across both spellings and every supported format.
    pub fn primary_variants() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for base in ["LICENCE", "LICENSE"] {
            paths.push(PathBuf::from(base));
            for ext in ["txt", "html", "md"] {
                paths.push(PathBuf::from(format!("{base}.{ext}")));
            }
        }
        paths
    }

    /// Additional-file names for one SPDX id, across basename and format.
    pub fn extra_variants(id: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for base in ["LICENCE", "LICENSE"] {
            for ext in ["txt", "html", "md"] {
                paths.push(PathBuf::from(format!("{base}-{id}.{ext}")));
            }
        }
        paths
    }

    /// All candidate filenames to check for existing licence files.
    pub fn candidates() -> &'static [&'static str] {
        &[
            "LICENSE",
            "LICENSE.txt",
            "LICENSE.md",
            "LICENCE",
            "LICENCE.txt",
            "LICENCE.md",
            "COPYING",
            "COPYING.txt",
        ]
    }
}

impl std::fmt::Display for LicenceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_explicit_license() {
        assert_eq!(LicenceName::resolve(Some("LICENSE")), LicenceName::License);
    }

    #[test]
    fn resolve_explicit_licence() {
        assert_eq!(LicenceName::resolve(Some("LICENCE")), LicenceName::Licence);
    }

    #[test]
    fn resolve_case_insensitive() {
        assert_eq!(LicenceName::resolve(Some("license")), LicenceName::License);
    }

    #[test]
    fn resolve_none_falls_back_to_detect() {
        // Just verify it doesn't panic — actual value depends on locale
        let _ = LicenceName::resolve(None);
    }

    #[test]
    fn file_path_txt() {
        assert_eq!(
            LicenceName::Licence.file_path("txt"),
            PathBuf::from("LICENCE.txt")
        );
    }

    #[test]
    fn file_path_html() {
        assert_eq!(
            LicenceName::License.file_path("html"),
            PathBuf::from("LICENSE.html")
        );
    }

    #[test]
    fn primary_and_extra_variants_cover_formats() {
        let primary = LicenceName::primary_variants();
        assert!(primary.contains(&PathBuf::from("LICENCE.txt")));
        assert!(primary.contains(&PathBuf::from("LICENSE.html")));
        assert!(primary.contains(&PathBuf::from("LICENCE")));
        let extras = LicenceName::extra_variants("Apache-2.0");
        assert!(extras.contains(&PathBuf::from("LICENCE-Apache-2.0.md")));
        assert!(extras.contains(&PathBuf::from("LICENSE-Apache-2.0.txt")));
        assert!(!extras.iter().any(|path| primary.contains(path)));
    }

    #[test]
    fn candidates_contain_common_names() {
        let c = LicenceName::candidates();
        assert!(c.contains(&"LICENSE"));
        assert!(c.contains(&"LICENCE"));
        assert!(c.contains(&"COPYING"));
    }
}
