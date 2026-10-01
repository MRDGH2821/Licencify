use anyhow::{Result, anyhow};
use regex::{NoExpand, Regex};
use std::path::Path;
use std::sync::LazyLock;

use crate::fs::{Fs, global_fs};

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

/// Result of an opt-in README update.
#[derive(Debug)]
pub struct ReadmeUpdate {
    /// Handwritten references that still name a different primary file.
    pub warnings: Vec<String>,
}

/// CLI value wins when present; otherwise use configuration. Default is off.
pub fn readme_updates_enabled(cli: Option<bool>, configured: Option<bool>) -> bool {
    cli.or(configured).unwrap_or(false)
}

/// Find an existing README file in the project root.
pub fn find_readme(fs: &dyn Fs) -> Option<&'static str> {
    README_CANDIDATES
        .iter()
        .copied()
        .find(|name| fs.exists(Path::new(name)))
}

/// Shields.io badge that links to the licence file just written.
pub fn badge_markdown(spdx_id: &str, licence_file: &str) -> String {
    let encoded = spdx_id.replace(' ', "%20");
    format!("[![License](https://img.shields.io/badge/License-{encoded}-blue.svg)]({licence_file})")
}

/// Licence section that links to the licence file just written.
pub fn license_section(spdx_id: &str, licence_file: &str) -> String {
    format!(
        "## License\n\nThis project is licensed under the [{spdx_id}]({licence_file}) licence.\n"
    )
}

/// Update a Markdown README so tool-managed links match `licence_path`.
///
/// Handwritten text is left in place. A missing or non-Markdown README is a
/// successful no-op. Read and write failures stay visible as errors.
pub fn update_readme(spdx_id: &str, licence_path: &Path) -> Result<ReadmeUpdate> {
    let fs = global_fs();
    let Some(readme_name) = find_readme(&*fs) else {
        return Ok(ReadmeUpdate {
            warnings: Vec::new(),
        });
    };
    // Only handle markdown-style READMEs in v1
    if !readme_name.ends_with(".md") && !readme_name.ends_with(".markdown") {
        return Ok(ReadmeUpdate {
            warnings: Vec::new(),
        });
    }

    let path = Path::new(readme_name);
    let content = fs
        .read_to_string(path)
        .ok_or_else(|| anyhow!("could not read {readme_name}"))?;
    let licence_file = licence_path.display().to_string();
    let badge = badge_markdown(spdx_id, &licence_file);
    let section = license_section(spdx_id, &licence_file);
    let (updated, changed) = apply_managed(&content, &badge, &section);
    let warnings = stale_manual_references(&updated, readme_name, &licence_file);
    if changed {
        fs.write(path, &updated)
            .map_err(|error| anyhow!("could not write {readme_name}: {error}"))?;
        println!("   Updated {readme_name} with license badge");
    }
    Ok(ReadmeUpdate { warnings })
}

/// Print README warnings without hiding a successful licence-file write.
pub fn report_readme(result: Result<ReadmeUpdate>) {
    match result {
        Ok(outcome) => {
            for warning in outcome.warnings {
                eprintln!("   Warning: {warning}");
            }
        }
        Err(error) => {
            eprintln!("   Warning: could not update README: {error}");
        }
    }
}

/// Replace the badge and section this tool generated. Append them only when
/// the README has no equivalent badge or licence heading.
fn apply_managed(content: &str, badge: &str, section: &str) -> (String, bool) {
    let had_managed_badge = managed_badge().is_match(content);
    let had_managed_section = managed_section().is_match(content);
    let mut next = content.to_string();
    if had_managed_badge {
        next = managed_badge()
            .replace_all(&next, NoExpand(badge))
            .into_owned();
    }
    if had_managed_section {
        next = managed_section()
            .replace_all(&next, NoExpand(section))
            .into_owned();
    }

    let visible = without_fences(content);
    let has_badge = had_managed_badge || other_badge().is_match(&visible);
    let has_section = had_managed_section || licence_heading().is_match(&visible);
    if !has_badge || !has_section {
        next = append_missing(
            next,
            if has_badge { None } else { Some(badge) },
            if has_section { None } else { Some(section) },
        );
    }
    let changed = next != content;
    (next, changed)
}

fn append_missing(body: String, badge: Option<&str>, section: Option<&str>) -> String {
    let mut extra = String::new();
    if let Some(badge) = badge {
        extra.push_str(badge);
    }
    if let Some(section) = section {
        if !extra.is_empty() {
            extra.push_str("\n\n");
        }
        extra.push_str(section);
    }
    if body.trim().is_empty() {
        return extra;
    }
    let mut body = body.trim_end().to_string();
    body.push_str("\n\n");
    body.push_str(&extra);
    if !body.ends_with('\n') {
        body.push('\n');
    }
    body
}

fn stale_manual_references(content: &str, readme_name: &str, licence_file: &str) -> Vec<String> {
    let visible = without_fences(content);
    let mut warnings = Vec::new();
    for target in link_targets().captures_iter(&visible) {
        let Some(name) = normalize_link_target(&target[1]) else {
            continue;
        };
        if !looks_like_primary(&name) || name == licence_file || same_file_name(&name, licence_file)
        {
            continue;
        }
        let warning = format!(
            "{readme_name} links to `{name}`, but the primary licence file is `{licence_file}`"
        );
        if !warnings.contains(&warning) {
            warnings.push(warning);
        }
    }
    warnings
}

fn same_file_name(link_name: &str, licence_file: &str) -> bool {
    Path::new(licence_file)
        .file_name()
        .and_then(|name| name.to_str())
        == Some(link_name)
}

fn normalize_link_target(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('#') {
        return None;
    }
    let target = if let Some(inner) = raw.strip_prefix('<') {
        inner.split('>').next().unwrap_or(inner)
    } else {
        raw.split_whitespace().next().unwrap_or(raw)
    };
    if target.contains("://") {
        return None;
    }
    let target = target.split(['?', '#']).next().unwrap_or(target);
    let name = target.rsplit(['/', '\\']).next().unwrap_or(target);
    let name = name.strip_prefix("./").unwrap_or(name);
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn looks_like_primary(name: &str) -> bool {
    let stem = match name.rsplit_once('.') {
        Some((stem, ext))
            if matches!(
                ext.to_ascii_lowercase().as_str(),
                "txt" | "html" | "md" | "markdown" | "rst"
            ) =>
        {
            stem
        }
        _ => name,
    };
    matches!(
        stem.to_ascii_uppercase().as_str(),
        "LICENCE" | "LICENSE" | "COPYING"
    )
}

fn without_fences(content: &str) -> String {
    fenced_block().replace_all(content, "").into_owned()
}

fn managed_badge() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\[!\[Licen[cs]e\]\(https://img\.shields\.io/badge/Licen[cs]e-[^)\s]+-blue\.svg\)\]\([^)\s]+\)",
        )
        .expect("badge pattern")
    });
    &RE
}

fn managed_section() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"## Licen[cs]e[ \t]*\r?\n\r?\nThis project is licensed under the \[[^\]]+\]\([^)\r\n]+\) licence\.[ \t]*\r?\n?",
        )
        .expect("section pattern")
    });
    &RE
}

fn other_badge() -> &'static Regex {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\[!\[licen[cs]e\]").expect("badge marker"));
    &RE
}

fn licence_heading() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?mi)^#{1,6}[ \t]+licen[cs]e[ \t]*$").expect("licence heading")
    });
    &RE
}

fn link_targets() -> &'static Regex {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\]\(([^)\n]+)\)").expect("markdown link"));
    &RE
}

fn fenced_block() -> &'static Regex {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)```.*?```|~~~.*?~~~").expect("code fence"));
    &RE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{Fs, FsGuard, MemFs};
    use std::sync::Arc;

    #[test]
    fn readme_override_enables_and_disables_config() {
        assert!(!readme_updates_enabled(None, None));
        assert!(readme_updates_enabled(None, Some(true)));
        assert!(!readme_updates_enabled(None, Some(false)));
        assert!(readme_updates_enabled(Some(true), Some(false)));
        assert!(!readme_updates_enabled(Some(false), Some(true)));
    }

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
    fn badge_and_section_link_to_the_given_filename() {
        let badge = badge_markdown("Apache-2.0", "LICENSE.html");
        let section = license_section("Apache-2.0", "LICENSE.html");
        assert!(badge.contains("Apache-2.0"));
        assert!(badge.contains("shields.io"));
        assert!(badge.contains("](LICENSE.html)"));
        assert!(!badge.contains("LICENCE.txt"));
        assert!(section.contains("[Apache-2.0](LICENSE.html)"));
    }

    #[test]
    fn update_readme_adds_section_for_actual_filename() {
        let (_guard, fs) = install_fs();
        fs.write(Path::new("README.md"), "# My Project").unwrap();
        let result = update_readme("MIT", Path::new("LICENSE.md")).unwrap();
        assert!(result.warnings.is_empty());
        let content = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(content.contains("# My Project"));
        assert!(content.contains("](LICENSE.md)"));
        assert!(content.contains("## License"));
        assert!(!content.contains("LICENCE.txt"));
    }

    #[test]
    fn generated_readme_content_is_idempotent() {
        let (_guard, fs) = install_fs();
        fs.write(Path::new("README.md"), "# My Project").unwrap();
        update_readme("MIT", Path::new("LICENCE.txt")).unwrap();
        let once = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(once.contains("](LICENCE.txt)"));
        update_readme("MIT", Path::new("LICENCE.txt")).unwrap();
        assert_eq!(fs.read_to_string(Path::new("README.md")).unwrap(), once);
    }

    #[test]
    fn licence_change_replaces_only_tool_managed_content() {
        let (_guard, fs) = install_fs();
        fs.write(
            Path::new("README.md"),
            "# Project\n\nHandwritten note stays.\n\n[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENCE.txt)\n\n## License\n\nThis project is licensed under the [MIT](LICENCE.txt) licence.\n",
        )
        .unwrap();
        update_readme("Apache-2.0", Path::new("LICENSE.html")).unwrap();
        let content = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(content.contains("Handwritten note stays."));
        assert!(content.contains("License-Apache-2.0-blue.svg"));
        assert!(content.contains("[Apache-2.0](LICENSE.html)"));
        assert!(!content.contains("License-MIT-blue.svg"));
        assert_eq!(content.matches("## License").count(), 1);
    }

    #[test]
    fn handwritten_section_survives_and_stale_link_warns() {
        let (_guard, fs) = install_fs();
        fs.write(
            Path::new("README.md"),
            "# Project\n\n## License\n\nKept by hand. See [the old file](LICENCE.txt).\n",
        )
        .unwrap();
        let result = update_readme("MIT", Path::new("LICENSE.md")).unwrap();
        let content = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(content.contains("Kept by hand. See [the old file](LICENCE.txt)."));
        assert_eq!(content.matches("## License").count(), 1);
        assert!(content.contains("](LICENSE.md)"));
        assert!(
            result.warnings.iter().any(
                |warning| warning.contains("`LICENCE.txt`") && warning.contains("`LICENSE.md`")
            )
        );
    }

    #[test]
    fn equivalent_tool_content_does_not_rewrite() {
        let (_guard, fs) = install_fs();
        let body = "# Project\n\n[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENCE.txt)\n\n## License\n\nThis project is licensed under the [MIT](LICENCE.txt) licence.\n";
        fs.write(Path::new("README.md"), body).unwrap();
        let result = update_readme("MIT", Path::new("LICENCE.txt")).unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(
            fs.read_to_string(Path::new("README.md")).as_deref(),
            Some(body)
        );
    }

    #[test]
    fn fenced_example_does_not_count_as_a_section_or_a_stale_link() {
        let (_guard, fs) = install_fs();
        fs.write(
            Path::new("README.md"),
            "# Project\n\n```\n## License\n[![License](LICENCE.txt)](LICENCE.txt)\n```\n",
        )
        .unwrap();
        let result = update_readme("MIT", Path::new("LICENSE.md")).unwrap();
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        let content = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(content.contains("```"));
        assert!(content.contains("This project is licensed under the [MIT](LICENSE.md)"));
    }

    #[test]
    fn update_readme_skips_if_no_readme() {
        let (_guard, _fs) = install_fs();
        let result = update_readme("MIT", Path::new("LICENCE.txt")).unwrap();
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn update_readme_skips_non_markdown() {
        let (_guard, fs) = install_fs();
        fs.write(Path::new("README.rst"), "Hello\n").unwrap();
        update_readme("MIT", Path::new("LICENCE.txt")).unwrap();
        assert_eq!(
            fs.read_to_string(Path::new("README.rst")).as_deref(),
            Some("Hello\n")
        );
    }

    #[test]
    fn read_error_stays_visible() {
        let _guard = FsGuard::new();
        crate::fs::set_global_fs(Arc::new(ReadmeUnreadable));
        let error = update_readme("MIT", Path::new("LICENCE.txt")).unwrap_err();
        assert!(error.to_string().contains("could not read README.md"));
    }

    #[test]
    fn write_error_stays_visible_and_keeps_previous_text() {
        let _guard = FsGuard::new();
        let inner = Arc::new(MemFs::new());
        inner.write(Path::new("README.md"), "# Keep\n").unwrap();
        crate::fs::set_global_fs(Arc::new(ReadmeWriteFail {
            inner: Arc::clone(&inner),
        }));
        let error = update_readme("MIT", Path::new("LICENCE.txt")).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("could not write README.md"), "{message}");
        assert!(message.contains("readme write failed"), "{message}");
        assert_eq!(
            inner.read_to_string(Path::new("README.md")).as_deref(),
            Some("# Keep\n")
        );
    }

    fn install_fs() -> (FsGuard, Arc<MemFs>) {
        let guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        (guard, fs)
    }

    struct ReadmeUnreadable;

    impl Fs for ReadmeUnreadable {
        fn read_to_string(&self, _path: &Path) -> Option<String> {
            None
        }

        fn write(&self, _path: &Path, _contents: &str) -> std::io::Result<()> {
            Ok(())
        }

        fn exists(&self, path: &Path) -> bool {
            path == Path::new("README.md")
        }

        fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }

        fn read_dir(&self, _path: &Path) -> Vec<std::path::PathBuf> {
            Vec::new()
        }

        fn remove_dir_all(&self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct ReadmeWriteFail {
        inner: Arc<MemFs>,
    }

    impl Fs for ReadmeWriteFail {
        fn read_to_string(&self, path: &Path) -> Option<String> {
            self.inner.read_to_string(path)
        }

        fn write(&self, path: &Path, contents: &str) -> std::io::Result<()> {
            if path.ends_with("README.md") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "readme write failed",
                ));
            }
            self.inner.write(path, contents)
        }

        fn exists(&self, path: &Path) -> bool {
            self.inner.exists(path)
        }

        fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.inner.create_dir_all(path)
        }

        fn read_dir(&self, path: &Path) -> Vec<std::path::PathBuf> {
            self.inner.read_dir(path)
        }

        fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.inner.remove_dir_all(path)
        }
    }
}
