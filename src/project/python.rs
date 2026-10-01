use super::handler::ManifestHandler;
use crate::fs::Fs;
use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::DocumentMut;

const PYPROJECT_TOML: &str = "pyproject.toml";

pub struct PythonHandler;

impl ManifestHandler for PythonHandler {
    fn name(&self) -> &str {
        PYPROJECT_TOML
    }

    fn exists(&self, fs: &dyn Fs) -> bool {
        fs.exists(Path::new(PYPROJECT_TOML))
    }

    fn update(&self, fs: &dyn Fs, license_id: &str) -> Result<()> {
        let content = fs
            .read_to_string(Path::new(PYPROJECT_TOML))
            .with_context(|| format!("failed to read {PYPROJECT_TOML}"))?;
        let mut doc: DocumentMut = content
            .parse()
            .with_context(|| format!("failed to parse {PYPROJECT_TOML} as TOML"))?;
        let notice = if super::is_proprietary_selection(license_id) {
            Some(super::require_primary_notice(fs)?)
        } else {
            None
        };
        let project = doc
            .get_mut("project")
            .and_then(|item| item.as_table_like_mut())
            .with_context(|| format!("{PYPROJECT_TOML} has no [project] table"))?;
        if let Some(notice) = notice {
            project.remove("license");
            include_notice_file(project, &notice)?;
            eprintln!(
                "Warning: pyproject.toml does not declare an SPDX licence for a proprietary notice. Private publication must be controlled separately."
            );
        } else {
            // Same SPDX representation this handler already writes for open-source IDs.
            let mut table = toml_edit::InlineTable::new();
            table.insert("text", toml_edit::Value::from(license_id));
            project.insert("license", toml_edit::value(table));
        }
        fs.write(Path::new(PYPROJECT_TOML), &doc.to_string())
            .with_context(|| format!("failed to write {PYPROJECT_TOML}"))?;
        Ok(())
    }
}

fn include_notice_file(project: &mut dyn toml_edit::TableLike, notice: &str) -> Result<()> {
    if let Some(existing) = project.get_mut("license-files") {
        let array = existing.as_array_mut().with_context(|| {
            format!("{PYPROJECT_TOML} project.license-files must be an array of filenames")
        })?;
        let present = array.iter().any(|value| value.as_str() == Some(notice));
        if !present {
            array.push(notice);
        }
        return Ok(());
    }
    let mut array = toml_edit::Array::new();
    array.push(notice);
    project.insert(
        "license-files",
        toml_edit::Item::Value(toml_edit::Value::Array(array)),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MemFs;

    fn sample_pyproject() -> &'static str {
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nlicense = { text = \"MIT\" }\n"
    }

    #[test]
    fn python_proprietary_removes_license_and_lists_notice() {
        let fs = MemFs::new();
        fs.write_file(Path::new(PYPROJECT_TOML), sample_pyproject());
        fs.write_file(Path::new("LICENCE.html"), "<p>notice</p>\n");
        PythonHandler.update(&fs, "proprietary").unwrap();

        let content = fs.read_to_string(Path::new(PYPROJECT_TOML)).unwrap();
        assert!(content.contains("name = \"demo\""), "{content}");
        assert!(content.contains("license-files"), "{content}");
        assert!(content.contains("LICENCE.html"), "{content}");
        assert!(!content.contains("license ="), "{content}");
        assert!(!content.contains("UNLICENSED"), "{content}");
        assert!(!content.contains("proprietary"), "{content}");
    }

    #[test]
    fn python_proprietary_keeps_existing_license_files() {
        let fs = MemFs::new();
        fs.write_file(
            Path::new(PYPROJECT_TOML),
            "[project]\nname = \"demo\"\nlicense-files = [\"NOTICE\"]\n",
        );
        fs.write_file(Path::new("LICENCE.txt"), "notice\n");
        PythonHandler.update(&fs, "proprietary").unwrap();
        PythonHandler.update(&fs, "proprietary").unwrap();

        let content = fs.read_to_string(Path::new(PYPROJECT_TOML)).unwrap();
        assert!(content.contains("NOTICE"), "{content}");
        assert_eq!(content.matches("LICENCE.txt").count(), 1, "{content}");
    }

    #[test]
    fn python_proprietary_without_notice_does_not_write() {
        let fs = MemFs::new();
        fs.write_file(Path::new(PYPROJECT_TOML), sample_pyproject());
        let error = PythonHandler.update(&fs, "proprietary").unwrap_err();
        assert!(
            error.to_string().contains("existing primary notice"),
            "{error}"
        );
        let content = fs.read_to_string(Path::new(PYPROJECT_TOML)).unwrap();
        assert!(content.contains("MIT"), "{content}");
        assert!(!content.contains("license-files"), "{content}");
    }

    #[test]
    fn python_malformed_license_files_are_not_rewritten() {
        let fs = MemFs::new();
        let original = "[project]\nname = \"demo\"\nlicense = { text = \"MIT\" }\nlicense-files = \"LICENCE\"\n";
        fs.write_file(Path::new(PYPROJECT_TOML), original);
        fs.write_file(Path::new("LICENCE.txt"), "notice\n");
        let error = PythonHandler.update(&fs, "proprietary").unwrap_err();
        assert!(error.to_string().contains("array"), "{error}");
        assert_eq!(
            fs.read_to_string(Path::new(PYPROJECT_TOML)).as_deref(),
            Some(original)
        );
    }

    #[test]
    fn python_spdx_switch_restores_license_and_keeps_notice_list() {
        let fs = MemFs::new();
        fs.write_file(
            Path::new(PYPROJECT_TOML),
            "[project]\nname = \"demo\"\nlicense-files = [\"LICENCE.txt\"]\n",
        );
        PythonHandler.update(&fs, "Apache-2.0").unwrap();

        let content = fs.read_to_string(Path::new(PYPROJECT_TOML)).unwrap();
        assert!(content.contains("Apache-2.0"), "{content}");
        assert!(content.contains("LICENCE.txt"), "{content}");
    }
}
