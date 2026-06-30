use anyhow::Result;
use std::path::Path;

use crate::fs::{global_fs, Fs};

/// README filenames to detect, ordered by preference.
const README_CANDIDATES: &[&str] = &[
    "README.md",
    "README.markdown",
    "README.rst",
    "README.txt",
    "README",
    "Readme.md",
    "readme.md",
];

/// Find an existing README file in the project root.
pub fn find_readme(fs: &dyn Fs) -> Option<&'static str> {
    README_CANDIDATES
        .iter()
        .copied()
        .find(|name| fs.exists(Path::new(name)))
}

/// Generate a shields.io badge URL for the given SPDX ID.
pub fn badge_url(spdx_id: &str) -> String {
    let encoded = spdx_id.replace(' ', "%20");
    format!(
        "[![License](https://img.shields.io/badge/License-{}-blue.svg)](LICENCE.txt)",
        encoded
    )
}

/// Generate the "License" section text to append to a README file.
pub fn license_section(spdx_id: &str) -> String {
    format!(
        "\n\n## License\n\nThis project is licensed under the [{}](LICENCE.txt) licence.\n",
        spdx_id
    )
}

/// Update the README with license badge and section (if README exists).
/// Returns true if the README was updated.
pub fn update_readme(spdx_id: &str) -> Result<bool> {
    let fs = global_fs();
    let readme_name = match find_readme(&*fs) {
        Some(name) => name,
        None => return Ok(false),
    };

    // Only handle markdown-style READMEs in v1
    if !readme_name.ends_with(".md") && !readme_name.ends_with(".markdown") {
        return Ok(false);
    }

    let content = fs
        .read_to_string(Path::new(readme_name))
        .unwrap_or_default();

    // Idempotency: skip if already has license section or badge
    if content.contains("## License") || content.contains("[![License]") {
        return Ok(false);
    }

    let badge = badge_url(spdx_id);
    let section = license_section(spdx_id);
    let updated = format!("{}\n\n{}{}", content.trim(), badge, section);
    fs.write(Path::new(readme_name), &updated)?;
    println!("   Updated {} with license badge", readme_name);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{FsGuard, MemFs};
    use std::sync::Arc;

    #[test]
    fn find_readme_returns_none_on_empty_fs() {
        let fs = MemFs::new();
        assert!(find_readme(&fs).is_none());
    }

    #[test]
    fn find_readme_finds_readme_md() {
        let fs = MemFs::new();
        fs.write(Path::new("README.md"), "# Hello").unwrap();
        assert_eq!(find_readme(&fs), Some("README.md"));
    }

    #[test]
    fn find_readme_prefers_md_over_txt() {
        let fs = MemFs::new();
        fs.write(Path::new("README.md"), "# Hello").unwrap();
        fs.write(Path::new("README.txt"), "Hello").unwrap();
        assert_eq!(find_readme(&fs), Some("README.md"));
    }

    #[test]
    fn badge_url_contains_spdx_id() {
        let url = badge_url("MIT");
        assert!(url.contains("MIT"));
        assert!(url.contains("shields.io"));
        assert!(url.contains("LICENCE.txt"));
    }

    #[test]
    fn update_readme_adds_section() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("README.md"), "# My Project").unwrap();
        let result = update_readme("MIT").unwrap();
        assert!(result);
        let content = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(content.contains("MIT"));
        assert!(content.contains("shields.io"));
        assert!(content.contains("## License"));
    }

    #[test]
    fn update_readme_skips_if_already_has_license() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("README.md"), "# Project\n\n## License\nMIT")
            .unwrap();
        let result = update_readme("Apache-2.0").unwrap();
        assert!(!result);
    }

    #[test]
    fn update_readme_skips_if_no_readme() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        let result = update_readme("MIT").unwrap();
        assert!(!result);
    }

    #[test]
    fn update_readme_skips_non_markdown() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("README.rst"), "Hello").unwrap();
        let result = update_readme("MIT").unwrap();
        assert!(!result);
    }
}
