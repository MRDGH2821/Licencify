use crate::{cli::LicenseFormat, project, provider, resolution};

pub fn cmd_update(
    spdx: &str,
    author: Option<String>,
    company: Option<String>,
    email: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
    yes: bool,
    permit_promotion: bool,
    update_readme: Option<bool>,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = crate::config::Config::load_effective(None)?;
    let update_readme =
        crate::readme::readme_updates_enabled(update_readme, config.default.update_readme);
    resolution::resolve_author(author.clone(), Some(&config))?;
    let spdx = super::generate::selected_licence_id(Some(spdx), None, true)?;
    let info = prov.info(&spdx)?;
    let ctx = resolution::resolve_context(
        &spdx,
        author,
        year,
        company,
        email,
        Some(&config),
        &prov,
        &format,
    )?;
    let additional = config
        .default
        .additional_licences
        .clone()
        .unwrap_or_default();
    let plan =
        super::generate::plan_primary(&ctx, &spdx, &info.id, &additional, permit_promotion, true)?;

    if !super::generate::confirm_proceed(
        yes,
        &format!(
            "About to replace the primary licence with {} ({}). Continue? [Y/n] ",
            info.name, info.id
        ),
        true,
    )? {
        println!("Aborted.");
        return Ok(());
    }

    let saved = super::generate::commit_primary(&plan, &info.id, &ctx.author)?;
    if spdx.eq_ignore_ascii_case("proprietary") || info.id == "UNLICENSED" {
        println!("✅ Updated proprietary notice as {}", plan.path.display());
    } else {
        println!(
            "✅ Updated {} ({}) [from {}] as {}",
            info.name,
            info.id,
            plan.template_source,
            plan.path.display()
        );
    }
    if saved {
        println!("   Updated project config defaults");
    }

    match project::update_manifest(&info.id, &ctx.author, &ctx.year) {
        Ok(files) if !files.is_empty() => {
            println!("   Updated: {}", files.join(", "));
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("   Warning: could not update project manifests: {}", e);
        }
    }

    if update_readme {
        crate::readme::report_readme(crate::readme::update_readme(&info.id, &plan.path));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LicenseFormat;
    use crate::commands::add::cmd_add;
    use crate::fs::{Fs, FsGuard, MemFs};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    fn update(
        spdx: &str,
        author: &str,
        format: LicenseFormat,
        permit_promotion: bool,
    ) -> anyhow::Result<()> {
        cmd_update(
            spdx,
            Some(author.into()),
            None,
            None,
            Some("2024".into()),
            format,
            true,
            permit_promotion,
            None,
        )
    }

    #[test]
    fn cmd_update_requires_existing_file() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        let result = update("MIT", "Test Author", LicenseFormat::Txt, false);
        assert!(result.is_err());
    }

    #[test]
    fn cmd_update_requires_replacement_id() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("LICENCE.txt"), "keep").unwrap();
        let result = update("   ", "Test Author", LicenseFormat::Txt, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("replacement"));
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE.txt")).as_deref(),
            Some("keep")
        );
    }

    #[test]
    fn cmd_update_replaces_content() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let add_result = cmd_add(
            Some("MIT"),
            Some("Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            true,
            false,
            None,
        );
        assert!(add_result.is_ok());
        let result = update("Apache-2.0", "Author", LicenseFormat::Txt, false);
        assert!(result.is_ok(), "cmd_update failed: {:?}", result.err());
    }

    #[test]
    fn cmd_update_writes_expected_mit_text() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs.write(Path::new("LICENCE.txt"), "previous licence text")
            .unwrap();
        let result = update("MIT", "Test Author", LicenseFormat::Txt, false);
        assert!(result.is_ok(), "cmd_update failed: {:?}", result.err());
        let text = fs
            .read_to_string(Path::new("LICENCE.txt"))
            .expect("LICENCE.txt");
        assert!(text.contains("MIT License"));
        assert!(text.contains("Copyright (c) 2024 Test Author"));
        assert!(!text.contains("previous licence text"));
    }

    #[test]
    fn ambiguous_primaries_cause_no_changes() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("LICENCE.txt"), "licence").unwrap();
        fs.write(Path::new("LICENSE.md"), "license").unwrap();
        let result = update("MIT", "Test Author", LicenseFormat::Html, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not changing"));
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE.txt")).as_deref(),
            Some("licence")
        );
        assert_eq!(
            fs.read_to_string(Path::new("LICENSE.md")).as_deref(),
            Some("license")
        );
        assert!(!fs.exists(Path::new("LICENCE.html")));
        assert!(!fs.exists(Path::new("LICENSE.html")));
    }

    #[test]
    fn format_change_removes_old_primary_only_after_replacement() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("LICENCE.txt"), "old primary").unwrap();
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"Apache-2.0\"\nlicence_file_name = \"licence\"\nformat = \"txt\"\n",
        )
        .unwrap();
        let result = update("MIT", "Test Author", LicenseFormat::Html, false);
        assert!(result.is_ok(), "cmd_update failed: {:?}", result.err());
        assert!(!fs.exists(Path::new("LICENCE.txt")));
        let text = fs
            .read_to_string(Path::new("LICENCE.html"))
            .expect("LICENCE.html");
        assert!(text.contains("MIT"));
        assert!(!text.contains("old primary"));
    }

    #[test]
    fn yes_does_not_authorize_promotion() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("LICENCE.txt"), "old primary").unwrap();
        fs.write(Path::new("LICENCE-MIT.html"), "EXTRA-BODY")
            .unwrap();
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"Apache-2.0\"\nauthor = \"Test Author\"\nadditional-licences = [\"MIT\"]\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        let result = update("MIT", "Test Author", LicenseFormat::Txt, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("--permit-promotion")
        );
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE.txt")).as_deref(),
            Some("old primary")
        );
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE-MIT.html")).as_deref(),
            Some("EXTRA-BODY")
        );
    }

    #[test]
    fn promotion_keeps_source_format_and_removes_one_extra() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("LICENCE.txt"), "old primary").unwrap();
        fs.write(Path::new("LICENCE-MIT.html"), "EXTRA-BODY")
            .unwrap();
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"Apache-2.0\"\nauthor = \"Test Author\"\nadditional-licences = [\"MIT\"]\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        let result = update("MIT", "Test Author", LicenseFormat::Txt, true);
        assert!(result.is_ok(), "promotion failed: {:?}", result.err());
        assert!(!fs.exists(Path::new("LICENCE.txt")));
        assert!(!fs.exists(Path::new("LICENCE-MIT.html")));
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE.html")).as_deref(),
            Some("EXTRA-BODY")
        );
        assert!(!fs.exists(Path::new("LICENCE.txt")));
        let saved = fs.read_to_string(&config).unwrap();
        let additional_line = saved
            .lines()
            .find(|line| line.contains("additional"))
            .unwrap_or("");
        assert!(!additional_line.contains("MIT"), "{saved}");
        assert!(saved.contains("MIT"), "{saved}");
    }

    #[test]
    fn multiple_matching_extras_cause_no_changes() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(Path::new("LICENCE.txt"), "old primary").unwrap();
        fs.write(Path::new("LICENCE-MIT.txt"), "one").unwrap();
        fs.write(Path::new("LICENSE-MIT.html"), "two").unwrap();
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"Apache-2.0\"\nadditional-licences = [\"MIT\"]\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        let result = update("MIT", "Test Author", LicenseFormat::Txt, true);
        assert!(result.is_err());
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE.txt")).as_deref(),
            Some("old primary")
        );
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE-MIT.txt")).as_deref(),
            Some("one")
        );
        assert_eq!(
            fs.read_to_string(Path::new("LICENSE-MIT.html")).as_deref(),
            Some("two")
        );
    }

    #[test]
    fn failed_config_update_preserves_old_primary_and_staged_output() {
        let _guard = FsGuard::new();
        let inner = Arc::new(MemFs::new());
        inner
            .write(Path::new("LICENCE.txt"), "old primary")
            .unwrap();
        inner
            .write(Path::new("LICENCE-MIT.html"), "EXTRA-BODY")
            .unwrap();
        let config = crate::config::Config::project_path().unwrap();
        inner
            .write(
                &config,
                "[default]\nlicence = \"Apache-2.0\"\nauthor = \"Test Author\"\nadditional-licences = [\"MIT\"]\nlicence_file_name = \"licence\"\n",
            )
            .unwrap();
        crate::fs::set_global_fs(Arc::new(FailConfigWrite(inner.clone())));
        let result = update("MIT", "Test Author", LicenseFormat::Txt, true);
        assert!(result.is_err());
        assert_eq!(
            inner.read_to_string(Path::new("LICENCE.txt")).as_deref(),
            Some("old primary")
        );
        assert_eq!(
            inner
                .read_to_string(Path::new("LICENCE-MIT.html"))
                .as_deref(),
            Some("EXTRA-BODY")
        );
        assert!(!inner.exists(Path::new("LICENCE.html")));
        assert_eq!(
            inner
                .read_to_string(Path::new("LICENCE.html.licencify-new"))
                .as_deref(),
            Some("EXTRA-BODY")
        );
        let saved = inner.read_to_string(&config).unwrap();
        assert!(saved.contains("Apache-2.0"));
        assert!(saved.contains("MIT"));
    }

    #[test]
    fn cmd_update_refreshes_tool_managed_readme_and_keeps_handwritten_text() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        let config = crate::config::Config::project_path().unwrap();
        fs.write(&config, "[default]\nlicence_file_name = \"licence\"\n")
            .unwrap();
        fs.write(Path::new("LICENCE.txt"), "old primary").unwrap();
        fs.write(
            Path::new("README.md"),
            "# Project\n\nHandwritten note stays.\n\n[![License](https://img.shields.io/badge/License-Apache-2.0-blue.svg)](LICENCE.txt)\n\n## License\n\nThis project is licensed under the [Apache-2.0](LICENCE.txt) licence.\n\nSee [the old file](LICENCE.txt).\n",
        )
        .unwrap();
        let result = cmd_update(
            "MIT",
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Html,
            true,
            false,
            Some(true),
        );
        assert!(result.is_ok(), "cmd_update failed: {:?}", result.err());
        assert!(fs.exists(Path::new("LICENCE.html")));
        assert!(!fs.exists(Path::new("LICENCE.txt")));
        let readme = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(readme.contains("Handwritten note stays."));
        assert!(readme.contains("See [the old file](LICENCE.txt)."));
        assert!(readme.contains("](LICENCE.html)"));
        assert!(readme.contains("[MIT](LICENCE.html)"));
        assert!(!readme.contains("Apache-2.0"));
        assert_eq!(readme.matches("## License").count(), 1);
    }

    #[test]
    fn cmd_update_no_update_readme_leaves_handwritten_readme() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nupdate_readme = true\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        fs.write(Path::new("LICENCE.txt"), "old primary").unwrap();
        fs.write(Path::new("README.md"), "# Keep\n").unwrap();
        let result = cmd_update(
            "MIT",
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
            Some(false),
        );
        assert!(result.is_ok(), "cmd_update failed: {:?}", result.err());
        assert_eq!(
            fs.read_to_string(Path::new("README.md")).as_deref(),
            Some("# Keep\n")
        );
        assert!(
            fs.read_to_string(Path::new("LICENCE.txt"))
                .unwrap()
                .contains("MIT")
        );
    }

    struct FailConfigWrite(Arc<MemFs>);

    impl Fs for FailConfigWrite {
        fn read_to_string(&self, path: &Path) -> Option<String> {
            self.0.read_to_string(path)
        }

        fn write(&self, path: &Path, contents: &str) -> std::io::Result<()> {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if path
                .components()
                .any(|component| component.as_os_str() == "licencify")
                && name.starts_with("config")
            {
                return Err(std::io::Error::other("config write failed"));
            }
            self.0.write(path, contents)
        }

        fn exists(&self, path: &Path) -> bool {
            self.0.exists(path)
        }

        fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.0.create_dir_all(path)
        }

        fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
            self.0.read_dir(path)
        }

        fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.0.remove_dir_all(path)
        }

        fn remove_file(&self, path: &Path) -> std::io::Result<()> {
            self.0.remove_file(path)
        }

        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            self.0.rename(from, to)
        }
    }
}
