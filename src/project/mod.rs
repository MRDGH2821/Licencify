use crate::fs::global_fs;
use crate::licence_name::LicenceName;
use anyhow::{Context, Result};

mod cargo;
pub mod handler;
mod npm;
mod python;

/// `proprietary` is Licencify's config value. `UNLICENSED` is only an incoming
/// alias from older callers and is never written into an SPDX licence field.
fn is_proprietary_selection(license_id: &str) -> bool {
    license_id.eq_ignore_ascii_case("proprietary") || license_id.eq_ignore_ascii_case("UNLICENSED")
}

/// The single existing primary notice filename. A proprietary manifest must not
/// point at a path that was not written, including under `--no-file`.
pub(crate) fn require_primary_notice(fs: &dyn crate::fs::Fs) -> Result<String> {
    let found: Vec<_> = LicenceName::primary_variants()
        .into_iter()
        .filter(|path| fs.exists(path))
        .collect();
    match found.as_slice() {
        [one] => one
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .context("primary notice path has no UTF-8 filename"),
        [] => anyhow::bail!(
            "proprietary manifest update requires an existing primary notice; refusing to reference a file that does not exist"
        ),
        many => anyhow::bail!(
            "Multiple primary licence files ({}); not updating manifests",
            many.iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Update the license field in every manifest found in the current directory.
///
/// Returns a list of manifests that were updated (e.g. `["Cargo.toml", "package.json"]`).
/// A proprietary selection marks manifests private without writing `proprietary` or
/// `UNLICENSED` as an SPDX expression. Switching back to an SPDX ID leaves Cargo
/// `publish = false` and npm `private = true` in place.
pub fn update_manifest(
    license_id: &str,
    _author: &str,
    _year: &str,
    additional: &[String],
) -> Result<Vec<String>> {
    if !additional.is_empty() {
        eprintln!(
            "Warning: skipping manifest updates; additional-licences does not specify whether the IDs combine with OR or AND"
        );
        return Ok(Vec::new());
    }
    let fs = global_fs();
    let mut updated = Vec::new();
    let mut failures = Vec::new();
    for handler in handlers() {
        if !handler.exists(&*fs) {
            continue;
        }
        match handler.update(&*fs, license_id) {
            Ok(()) => updated.push(handler.name().to_string()),
            Err(error) => failures.push(format!("{}: {error:#}", handler.name())),
        }
    }
    if !failures.is_empty() {
        let updated_note = if updated.is_empty() {
            String::new()
        } else {
            format!("Updated {}. ", updated.join(", "))
        };
        anyhow::bail!(
            "{updated_note}Failed manifest updates: {}",
            failures.join("; ")
        );
    }
    Ok(updated)
}

fn handlers() -> Vec<Box<dyn handler::ManifestHandler>> {
    vec![
        Box::new(cargo::CargoHandler),
        Box::new(npm::NpmHandler),
        Box::new(python::PythonHandler),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{Fs, FsGuard, MemFs};
    use std::path::Path;
    use std::sync::Arc;

    fn install(files: &[(&str, &str)]) -> Arc<dyn Fs> {
        let fs = Arc::new(MemFs::new());
        for (path, contents) in files {
            fs.write(Path::new(path), contents).unwrap();
        }
        crate::fs::set_global_fs(fs.clone());
        fs
    }

    #[test]
    fn proprietary_marks_manifests_private_without_spdx_expression() {
        let _guard = FsGuard::new();
        let fs = install(&[
            (
                "Cargo.toml",
                "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\n",
            ),
            (
                "package.json",
                "{\n  \"name\": \"demo\",\n  \"license\": \"MIT\"\n}\n",
            ),
            (
                "pyproject.toml",
                "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nlicense = { text = \"MIT\" }\n",
            ),
            ("LICENCE.txt", "All Rights Reserved\n"),
        ]);

        let updated = update_manifest("proprietary", "Author", "2024", &[]).unwrap();
        assert_eq!(
            updated,
            vec![
                "Cargo.toml".to_string(),
                "package.json".to_string(),
                "pyproject.toml".to_string()
            ]
        );

        let cargo = fs.read_to_string(Path::new("Cargo.toml")).unwrap();
        assert!(cargo.contains("publish = false"), "{cargo}");
        assert!(cargo.contains("license-file = \"LICENCE.txt\""), "{cargo}");
        assert!(!cargo.contains("UNLICENSED"), "{cargo}");
        assert!(!cargo.contains("proprietary"), "{cargo}");
        assert!(!cargo.contains("license ="), "{cargo}");

        let package = fs.read_to_string(Path::new("package.json")).unwrap();
        assert!(package.contains("\"private\": true"), "{package}");
        assert!(package.contains("SEE LICENSE IN LICENCE.txt"), "{package}");
        assert!(!package.contains("UNLICENSED"), "{package}");
        assert!(!package.contains("proprietary"), "{package}");

        let python = fs.read_to_string(Path::new("pyproject.toml")).unwrap();
        assert!(python.contains("license-files"), "{python}");
        assert!(python.contains("LICENCE.txt"), "{python}");
        assert!(!python.contains("license ="), "{python}");
        assert!(!python.contains("UNLICENSED"), "{python}");
        assert!(!python.contains("proprietary"), "{python}");
    }

    #[test]
    fn missing_notice_does_not_reference_a_file() {
        let _guard = FsGuard::new();
        let fs = install(&[(
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\n",
        )]);
        let error = update_manifest("UNLICENSED", "Author", "2024", &[]).unwrap_err();
        assert!(
            error.to_string().contains("existing primary notice"),
            "{error}"
        );
        let cargo = fs.read_to_string(Path::new("Cargo.toml")).unwrap();
        assert!(cargo.contains("license = \"MIT\""), "{cargo}");
        assert!(!cargo.contains("license-file"), "{cargo}");
        assert!(!cargo.contains("publish"), "{cargo}");
    }

    #[test]
    fn spdx_switch_keeps_private_flags() {
        let _guard = FsGuard::new();
        let fs = install(&[
            (
                "Cargo.toml",
                "[package]\nname = \"demo\"\nversion = \"0.1.0\"\npublish = false\nlicense-file = \"LICENCE.txt\"\n",
            ),
            (
                "package.json",
                "{\n  \"name\": \"demo\",\n  \"private\": true,\n  \"license\": \"SEE LICENSE IN LICENCE.txt\"\n}\n",
            ),
            ("LICENCE.txt", "notice\n"),
        ]);

        update_manifest("MIT", "Author", "2024", &[]).unwrap();

        let cargo = fs.read_to_string(Path::new("Cargo.toml")).unwrap();
        assert!(cargo.contains("publish = false"), "{cargo}");
        assert!(cargo.contains("license = \"MIT\""), "{cargo}");
        assert!(!cargo.contains("license-file"), "{cargo}");

        let package = fs.read_to_string(Path::new("package.json")).unwrap();
        assert!(package.contains("\"private\": true"), "{package}");
        assert!(package.contains("\"license\": \"MIT\""), "{package}");
        assert!(!package.contains("SEE LICENSE IN"), "{package}");
    }
}
