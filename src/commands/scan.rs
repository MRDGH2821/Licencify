use anyhow::Result;
use regex::Regex;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use crate::config::Config;
use crate::detect;

const STEMS: &[&str] = &["LICENCE", "LICENSE", "COPYING"];
const EXTENSIONS: &[&str] = &["txt", "md", "html"];
const README_NAMES: &[&str] = &[
    "README.md",
    "README.markdown",
    "README.rst",
    "README.txt",
    "README",
    "Readme.md",
    "readme.md",
];

static CLAIM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:spdx-license-identifier\s*:\s*|licen[cs]e\s*:\s*|licensed under (?:the )?)([A-Za-z0-9][A-Za-z0-9.+-]*)",
    )
    .expect("claim pattern")
});
static BADGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://img\.shields\.io/badge/License-([A-Za-z0-9.+-]+)-").expect("badge pattern")
});
static LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\[[^\]]*\]\(([^)\s]+)\)").expect("link pattern"));

struct ParsedName {
    claim: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Coverage {
    Visit,
    DescendOnly,
    Skip,
}

struct Found {
    rel: String,
    dir: String,
    primary_name: bool,
    identified: Option<String>,
    filename_claim: Option<String>,
}

struct ReadmeFound {
    rel: String,
    dir: String,
    claims: Vec<String>,
}

struct ReadmeClaims {
    ids: Vec<String>,
    bare: bool,
}

/// Report conventional licence files and explicit README claims.
/// Does not create, rewrite, or delete project files.
pub fn cmd_scan(expected_primary: Option<&str>) -> Result<()> {
    let (root, global, shared, local) = Config::load_stacked()?;
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let effective = Config::effective_in(&global, &shared, &local, "")?;
    let excludes = effective
        .scan
        .as_ref()
        .and_then(|scan| scan.exclude.clone())
        .unwrap_or_default();
    let subdir_paths: Vec<String> = effective
        .subdirs
        .unwrap_or_default()
        .into_iter()
        .map(|entry| entry.path)
        .collect();
    for exclude in &excludes {
        for subdir in &subdir_paths {
            if overlaps(exclude, subdir) {
                eprintln!(
                    "Warning: scan exclusion `{exclude}` overlaps configured subdir `{subdir}`; scanning that subdir anyway"
                );
            }
        }
    }

    let failed_flag = Arc::new(AtomicBool::new(false));
    let mut founds = Vec::new();
    let mut readmes = Vec::new();
    let mut failed = false;
    let mut builder = ignore::WalkBuilder::new(&root);
    let root_for_filter = root.clone();
    let excludes_for_filter = excludes.clone();
    let subdirs_for_filter = subdir_paths.clone();
    let failed_in_filter = Arc::clone(&failed_flag);
    builder
        .hidden(false)
        .parents(false)
        .follow_links(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .filter_entry(move |entry| {
            keep_entry(
                entry,
                &root_for_filter,
                &excludes_for_filter,
                &subdirs_for_filter,
                &failed_in_filter,
            )
        });

    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("Error: unreadable {err}");
                failed = true;
                continue;
            }
        };
        let Some(rel) = relative_slash(&root, entry.path()).filter(|rel| !rel.is_empty()) else {
            continue;
        };
        if coverage(&rel, &excludes, &subdir_paths) != Coverage::Visit {
            continue;
        }
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if let Some(parsed) = licence_filename(&name) {
            match std::fs::read(entry.path()) {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    founds.push(Found {
                        dir: parent_rel(&rel),
                        primary_name: parsed.claim.is_none(),
                        identified: detect::detect_license(&text).map(str::to_string),
                        filename_claim: parsed.claim,
                        rel,
                    });
                }
                Err(err) => {
                    eprintln!("Error: unreadable {rel}: {err}");
                    failed = true;
                }
            }
            continue;
        }
        if is_readme(&name) {
            match std::fs::read_to_string(entry.path()) {
                Ok(text) => {
                    let claims = readme_claims(&text);
                    if claims.ids.is_empty() && !claims.bare {
                        continue;
                    }
                    readmes.push(ReadmeFound {
                        dir: parent_rel(&rel),
                        claims: claims.ids,
                        rel,
                    });
                }
                Err(err) => {
                    eprintln!("Error: unreadable {rel}: {err}");
                    failed = true;
                }
            }
        }
    }
    failed |= failed_flag.load(Ordering::Relaxed);

    let mut check_dirs = vec![String::new()];
    check_dirs.extend(subdir_paths.iter().cloned());
    for dir in &check_dirs {
        let path = if dir.is_empty() {
            root.clone()
        } else {
            root.join(dir)
        };
        if !dir.is_empty() && !path.is_dir() {
            eprintln!("Warning: missing configured directory {dir}");
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let (primary, additional) =
            expectation_for(&global, &shared, &local, dir, expected_primary)?;
        let label = display_dir(dir);
        if let Some(primary) = &primary {
            let present = founds
                .iter()
                .any(|found| found.dir == *dir && found.primary_name);
            if !present {
                eprintln!("Warning: missing expected primary {primary} in {label}");
            }
        }
        for id in &additional {
            let present = founds.iter().any(|found| {
                found.dir == *dir
                    && (found
                        .identified
                        .as_ref()
                        .is_some_and(|got| same_id(got, id))
                        || found
                            .filename_claim
                            .as_ref()
                            .is_some_and(|got| same_id(got, id)))
            });
            if !present {
                eprintln!("Warning: missing expected additional {id} in {label}");
            }
        }
    }

    for found in &founds {
        if let (Some(identified), Some(claim)) = (&found.identified, &found.filename_claim) {
            if !same_id(identified, claim) {
                eprintln!(
                    "Error: conflict {}: identified {identified}, claimed {claim}",
                    found.rel
                );
                failed = true;
            }
        }
        let (primary, additional) =
            expectation_for(&global, &shared, &local, &found.dir, expected_primary)?;
        if expectation_active(&primary, &additional) {
            if let Some(identified) = &found.identified {
                if !in_expected(identified, &primary, &additional) {
                    eprintln!(
                        "Error: conflict {}: identified {identified} is not an expected licence",
                        found.rel
                    );
                    failed = true;
                }
            }
        }
    }
    for readme in &readmes {
        let identified: Vec<&str> = founds
            .iter()
            .filter(|found| found.dir == readme.dir)
            .filter_map(|found| found.identified.as_deref())
            .collect();
        if identified.is_empty() {
            continue;
        }
        for claim in &readme.claims {
            if !identified.iter().any(|id| same_id(id, claim)) {
                eprintln!(
                    "Error: conflict {}: claimed {claim} disagrees with identified licence text",
                    readme.rel
                );
                failed = true;
            }
        }
    }

    let mut lines = Vec::new();
    for found in &founds {
        if let Some(id) = &found.identified {
            lines.push(format!("{}: identified {id}", found.rel));
        } else if let Some(id) = &found.filename_claim {
            lines.push(format!("{}: claimed {id}", found.rel));
        } else {
            eprintln!("Warning: unknown licence content in {}", found.rel);
            lines.push(format!("{}: unknown", found.rel));
        }
    }
    for readme in &readmes {
        if readme.claims.is_empty() {
            lines.push(format!("{}: claimed", readme.rel));
        } else {
            for claim in &readme.claims {
                lines.push(format!("{}: claimed {claim}", readme.rel));
            }
        }
    }
    lines.sort();
    for line in lines {
        println!("{line}");
    }

    if failed {
        anyhow::bail!("scan found conflicting or unreadable licence evidence");
    }
    Ok(())
}

fn keep_entry(
    entry: &ignore::DirEntry,
    root: &Path,
    excludes: &[String],
    subdirs: &[String],
    failed: &AtomicBool,
) -> bool {
    let rel = relative_slash(root, entry.path()).unwrap_or_default();
    if !rel.is_empty() && is_symlink(entry.path()) {
        match std::fs::canonicalize(entry.path()) {
            Ok(canon) if canon.starts_with(root) => {}
            Ok(_) => {
                eprintln!("Warning: skipped symlink {rel} outside the project root");
                return false;
            }
            Err(err) => {
                eprintln!("Error: unreadable {rel}: {err}");
                failed.store(true, Ordering::Relaxed);
                return false;
            }
        }
    }
    coverage(&rel, excludes, subdirs) != Coverage::Skip
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

fn expectation_for(
    global: &Config,
    shared: &Config,
    local: &Config,
    dir: &str,
    primary_override: Option<&str>,
) -> Result<(Option<String>, Vec<String>)> {
    let config = Config::effective_in(global, shared, local, dir)?;
    let primary = primary_override
        .map(str::to_string)
        .or(config.default.license);
    let additional = config.default.additional_licences.unwrap_or_default();
    Ok((primary, additional))
}

fn expectation_active(primary: &Option<String>, additional: &[String]) -> bool {
    primary.is_some() || !additional.is_empty()
}

fn in_expected(id: &str, primary: &Option<String>, additional: &[String]) -> bool {
    primary
        .as_ref()
        .is_some_and(|expected| same_id(expected, id))
        || additional.iter().any(|expected| same_id(expected, id))
}

fn display_dir(dir: &str) -> &str {
    if dir.is_empty() { "." } else { dir }
}

fn same_id(left: &str, right: &str) -> bool {
    canon_id(left) == canon_id(right)
}

fn canon_id(id: &str) -> String {
    let lower = id.to_ascii_lowercase();
    if lower == "proprietary" || lower == "unlicensed" {
        "unlicensed".to_string()
    } else {
        lower
    }
}

fn licence_filename(name: &str) -> Option<ParsedName> {
    let stem = match name.rsplit_once('.') {
        Some((stem, ext))
            if EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(ext)) =>
        {
            stem
        }
        _ => name,
    };
    for base in STEMS {
        if stem.eq_ignore_ascii_case(base) {
            return Some(ParsedName { claim: None });
        }
        let prefix = format!("{base}-");
        if stem.len() > prefix.len() && stem[..prefix.len()].eq_ignore_ascii_case(&prefix) {
            let claim = stem[prefix.len()..].to_string();
            if claim.is_empty() {
                return None;
            }
            return Some(ParsedName { claim: Some(claim) });
        }
    }
    None
}

fn is_readme(name: &str) -> bool {
    README_NAMES.contains(&name)
}

fn readme_claims(text: &str) -> ReadmeClaims {
    let visible = strip_fences(text);
    let mut ids = Vec::new();
    let mut push_id = |id: &str| {
        if is_stopword(id) {
            return;
        }
        if !ids.iter().any(|existing: &String| same_id(existing, id)) {
            ids.push(id.to_string());
        }
    };
    for captures in CLAIM_RE.captures_iter(&visible) {
        if let Some(id) = captures.get(1) {
            push_id(id.as_str());
        }
    }
    for captures in BADGE_RE.captures_iter(&visible) {
        if let Some(id) = captures.get(1) {
            push_id(id.as_str());
        }
    }
    let mut bare = false;
    for captures in LINK_RE.captures_iter(&visible) {
        let Some(target) = captures.get(1) else {
            continue;
        };
        let target = target.as_str().split(['?', '#']).next().unwrap_or("");
        let name = Path::new(target)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(parsed) = licence_filename(&name) {
            if let Some(id) = parsed.claim {
                push_id(&id);
            } else {
                bare = true;
            }
        }
    }
    ReadmeClaims { ids, bare }
}

fn is_stopword(id: &str) -> bool {
    matches!(
        id.to_ascii_lowercase().as_str(),
        "a" | "an"
            | "and"
            | "file"
            | "licence"
            | "license"
            | "or"
            | "see"
            | "the"
            | "this"
            | "under"
    )
}

fn strip_fences(text: &str) -> String {
    let mut visible = String::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if !fenced {
            visible.push_str(line);
            visible.push('\n');
        }
    }
    visible
}

fn parent_rel(rel: &str) -> String {
    match rel.rfind('/') {
        Some(index) => rel[..index].to_string(),
        None => String::new(),
    }
}

fn relative_slash(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    if relative.as_os_str().is_empty() {
        return Some(String::new());
    }
    Some(
        relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn overlaps(left: &str, right: &str) -> bool {
    within(left, right) || within(right, left)
}

fn within(path: &str, ancestor: &str) -> bool {
    path == ancestor || path.starts_with(&format!("{ancestor}/"))
}

fn coverage(rel: &str, excludes: &[String], subdirs: &[String]) -> Coverage {
    if rel.is_empty() || !excludes.iter().any(|exclude| within(rel, exclude)) {
        return Coverage::Visit;
    }
    let protected = |subdir: &String| excludes.iter().any(|exclude| overlaps(exclude, subdir));
    if subdirs
        .iter()
        .any(|subdir| protected(subdir) && within(rel, subdir))
    {
        return Coverage::Visit;
    }
    if subdirs
        .iter()
        .any(|subdir| protected(subdir) && within(subdir, rel) && subdir != rel)
    {
        return Coverage::DescendOnly;
    }
    Coverage::Skip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_suffixed_licence_names() {
        let plain = licence_filename("LICENSE").unwrap();
        assert!(plain.claim.is_none());
        let extra = licence_filename("LICENCE-GPL-3.0-only.txt").unwrap();
        assert_eq!(extra.claim.as_deref(), Some("GPL-3.0-only"));
        assert!(licence_filename("COPYING.md").unwrap().claim.is_none());
        assert!(licence_filename("notes.txt").is_none());
    }

    #[test]
    fn readme_claims_ignore_fenced_examples() {
        let text = "License: MIT\n\n```\nLicense: GPL-3.0-only\n```\n[licence](LICENCE.txt)\n";
        let claims = readme_claims(text);
        assert_eq!(claims.ids, vec!["MIT".to_string()]);
        assert!(claims.bare);
    }

    #[test]
    fn overlapping_subdir_stays_covered() {
        let excludes = vec!["vendor".to_string()];
        let subdirs = vec!["vendor/lib".to_string()];
        assert_eq!(
            coverage("vendor", &excludes, &subdirs),
            Coverage::DescendOnly
        );
        assert_eq!(
            coverage("vendor/lib/LICENSE", &excludes, &subdirs),
            Coverage::Visit
        );
        assert_eq!(
            coverage("vendor/other", &excludes, &subdirs),
            Coverage::Skip
        );
        let same = vec!["vendor".to_string()];
        assert_eq!(
            coverage("vendor/LICENSE", &excludes, &same),
            Coverage::Visit
        );
    }
}
