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
    fn candidates_contain_common_names() {
        let c = LicenceName::candidates();
        assert!(c.contains(&"LICENSE"));
        assert!(c.contains(&"LICENCE"));
        assert!(c.contains(&"COPYING"));
    }
}
