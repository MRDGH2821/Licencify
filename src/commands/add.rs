use crate::{cli::LicenseFormat, fs::global_fs, project, provider, resolution};
use std::io::IsTerminal;

pub fn cmd_add(
    spdx: Option<&str>,
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
    let spdx =
        super::generate::selected_licence_id(spdx, config.default.license.as_deref(), false)?;
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
        super::generate::plan_primary(&ctx, &spdx, &info.id, &additional, permit_promotion, false)?;

    if !yes && std::io::stdin().is_terminal() {
        println!("About to add license: {} ({})", info.name, info.id);
        println!("  Author:  {}", ctx.author);
        if let Some(ref company) = ctx.company {
            println!("  Company: {}", company);
        }
        if let Some(ref email) = ctx.email {
            println!("  Email:   {}", email);
        }
        println!("  Format:  {}", plan.format);
        println!();
    }
    if !super::generate::confirm_proceed(yes, "Continue? [Y/n] ", true)? {
        println!("Aborted.");
        return Ok(());
    }
    let fs = global_fs();
    if fs.exists(&plan.path)
        && !super::generate::confirm_proceed(
            yes,
            &format!("{} exists. Overwrite? [y/N] ", plan.path.display()),
            false,
        )?
    {
        println!("Aborted.");
        return Ok(());
    }

    let saved = super::generate::commit_primary(&plan, &info.id, &ctx.author)?;
    if spdx.eq_ignore_ascii_case("proprietary") || info.id == "UNLICENSED" {
        println!("✅ Added proprietary notice as {}", plan.path.display());
    } else {
        println!(
            "✅ Added {} ({}) [from {}] as {}",
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
    use crate::fs::{Fs, FsGuard, MemFs};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    fn add_mit(fs_author: &str) -> anyhow::Result<()> {
        cmd_add(
            Some("MIT"),
            Some(fs_author.into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            true,
            false,
            None,
        )
    }

    #[test]
    fn cmd_add_mit_returns_ok() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let result = add_mit("Test Author");
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        let expected_path = std::path::Path::new("LICENCE.txt");
        assert!(fs.exists(expected_path), "LICENCE.txt not written");
    }

    #[test]
    fn cmd_add_proprietary_returns_ok() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let result = cmd_add(
            Some("proprietary"),
            Some("Acme Corp".into()),
            Some("Acme Corp".into()),
            Some("legal@acme.com".into()),
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
            None,
        );
        assert!(
            result.is_ok(),
            "cmd_add proprietary failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn cmd_add_writes_expected_mit_text() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let result = cmd_add(
            Some("MIT"),
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
            None,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        let text = fs
            .read_to_string(std::path::Path::new("LICENCE.txt"))
            .expect("LICENCE.txt");
        assert!(text.contains("MIT License"));
        assert!(text.contains("Copyright (c) 2024 Test Author"));
    }

    #[test]
    fn cmd_add_without_id_uses_effective_licence() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"MIT\"\nauthor = \"From Config\"\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        let result = cmd_add(
            None,
            None,
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
            None,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        let text = fs
            .read_to_string(std::path::Path::new("LICENCE.txt"))
            .expect("LICENCE.txt");
        assert!(text.contains("MIT License"));
        assert!(text.contains("From Config"));
    }

    #[test]
    fn cmd_add_without_id_or_config_does_not_mutate() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(std::path::Path::new("LICENCE.txt"), "keep")
            .unwrap();
        let result = cmd_add(
            None,
            Some("Test Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            true,
            false,
            None,
        );
        assert!(result.is_err());
        assert_eq!(
            fs.read_to_string(std::path::Path::new("LICENCE.txt"))
                .as_deref(),
            Some("keep")
        );
    }

    #[test]
    fn invalid_licence_does_not_mutate() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(std::path::Path::new("LICENCE.txt"), "keep")
            .unwrap();
        let result = cmd_add(
            Some("not a licence"),
            Some("Test Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            true,
            false,
            None,
        );
        assert!(result.is_err());
        assert_eq!(
            fs.read_to_string(std::path::Path::new("LICENCE.txt"))
                .as_deref(),
            Some("keep")
        );
        assert!(!fs.exists(std::path::Path::new("LICENCE.txt.licencify-new")));
    }

    fn add_mit_readme(format: LicenseFormat, update_readme: Option<bool>) -> anyhow::Result<()> {
        cmd_add(
            Some("MIT"),
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            format,
            true,
            false,
            update_readme,
        )
    }

    #[test]
    fn cmd_add_update_readme_links_generated_file_and_is_idempotent() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        let config = crate::config::Config::project_path().unwrap();
        fs.write(&config, "[default]\nlicence_file_name = \"LICENSE\"\n")
            .unwrap();
        fs.write(Path::new("README.md"), "# Project\n").unwrap();
        let result = add_mit_readme(LicenseFormat::Html, Some(true));
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        assert!(fs.exists(Path::new("LICENSE.html")));
        let readme = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(readme.contains("](LICENSE.html)"));
        assert!(readme.contains("# Project"));
        assert!(!readme.contains("LICENCE.txt"));
        let again = add_mit_readme(LicenseFormat::Html, Some(true));
        assert!(again.is_ok(), "second add failed: {:?}", again.err());
        assert_eq!(
            fs.read_to_string(Path::new("README.md")).as_deref(),
            Some(readme.as_str())
        );
    }

    #[test]
    fn config_enables_readme_and_cli_can_disable_it() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nupdate_readme = true\nlicence_file_name = \"LICENCE\"\n",
        )
        .unwrap();
        fs.write(Path::new("README.md"), "# From config\n").unwrap();
        let enabled = add_mit_readme(LicenseFormat::Txt, None);
        assert!(enabled.is_ok(), "config enable failed: {:?}", enabled.err());
        let readme = fs.read_to_string(Path::new("README.md")).unwrap();
        assert!(readme.contains("](LICENCE.txt)"));

        fs.write(Path::new("README.md"), "# Keep\n").unwrap();
        let disabled = add_mit_readme(LicenseFormat::Txt, Some(false));
        assert!(disabled.is_ok(), "cli disable failed: {:?}", disabled.err());
        assert_eq!(
            fs.read_to_string(Path::new("README.md")).as_deref(),
            Some("# Keep\n")
        );
        assert!(fs.exists(Path::new("LICENCE.txt")));
    }

    #[test]
    fn missing_or_non_markdown_readme_is_success() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
        crate::fs::set_global_fs(fs.clone());
        let config = crate::config::Config::project_path().unwrap();
        fs.write(&config, "[default]\nlicence_file_name = \"LICENCE\"\n")
            .unwrap();
        let missing = add_mit_readme(LicenseFormat::Txt, Some(true));
        assert!(
            missing.is_ok(),
            "missing readme failed: {:?}",
            missing.err()
        );
        assert!(fs.exists(Path::new("LICENCE.txt")));

        fs.write(Path::new("README.rst"), "Hello\n").unwrap();
        let skipped = add_mit_readme(LicenseFormat::Txt, Some(true));
        assert!(skipped.is_ok(), "rst readme failed: {:?}", skipped.err());
        assert_eq!(
            fs.read_to_string(Path::new("README.rst")).as_deref(),
            Some("Hello\n")
        );
    }

    #[test]
    fn readme_write_error_stays_visible_without_losing_the_licence() {
        let _guard = FsGuard::new();
        let inner = Arc::new(MemFs::new());
        let config = crate::config::Config::project_path().unwrap();
        inner
            .write(&config, "[default]\nlicence_file_name = \"LICENCE\"\n")
            .unwrap();
        inner.write(Path::new("README.md"), "# Keep\n").unwrap();
        crate::fs::set_global_fs(Arc::new(ReadmeWriteFail {
            inner: Arc::clone(&inner),
        }));
        let result = add_mit_readme(LicenseFormat::Txt, Some(true));
        assert!(
            result.is_ok(),
            "licence write should still succeed: {:?}",
            result.err()
        );
        let licence = inner.read_to_string(Path::new("LICENCE.txt")).unwrap();
        assert!(licence.contains("MIT License"));
        assert_eq!(
            inner.read_to_string(Path::new("README.md")).as_deref(),
            Some("# Keep\n")
        );
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
            Fs::write(&*self.inner, path, contents)
        }

        fn exists(&self, path: &Path) -> bool {
            self.inner.exists(path)
        }

        fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.inner.create_dir_all(path)
        }

        fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
            self.inner.read_dir(path)
        }

        fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.inner.remove_dir_all(path)
        }

        fn remove_file(&self, path: &Path) -> std::io::Result<()> {
            self.inner.remove_file(path)
        }

        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            self.inner.rename(from, to)
        }
    }
}
