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
    no_file: bool,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = crate::config::Config::load_effective(None)?;
    let update_readme =
        crate::readme::readme_updates_enabled(update_readme, config.default.update_readme);
    resolution::resolve_author(author.clone(), Some(&config))?;
    let spdx =
        super::generate::selected_licence_id(spdx, config.default.license.as_deref(), false)?;
    let info = prov.info(&spdx)?;
    let recorded = resolution::canonical_licence_id(&spdx, &info.id);
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
    let extra_ids = plan
        .additional_after
        .as_deref()
        .unwrap_or(additional.as_slice());
    let extras = super::generate::plan_missing_extras(&ctx, &config, &prov, extra_ids, &format)?;

    if !yes && std::io::stdin().is_terminal() {
        println!("About to add license: {} ({})", info.name, recorded);
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
    if !no_file
        && fs.exists(&plan.path)
        && !super::generate::confirm_proceed(
            yes,
            &format!("{} exists. Overwrite? [y/N] ", plan.path.display()),
            false,
        )?
    {
        println!("Aborted.");
        return Ok(());
    }

    if no_file && recorded == "proprietary" {
        project::require_primary_notice(&*fs)?;
    }
    let saved =
        super::generate::publish_licence(no_file, &plan, &extras.missing, &recorded, &ctx.author)?;
    if no_file {
        println!("   Skipped licence files (--no-file)");
    } else if recorded == "proprietary" {
        println!("✅ Added proprietary notice as {}", plan.path.display());
    } else {
        println!(
            "✅ Added {} ({}) [from {}] as {}",
            info.name,
            recorded,
            plan.template_source,
            plan.path.display()
        );
    }
    if saved {
        println!("   Updated project config defaults");
    }
    if !no_file {
        for extra in &extras.missing {
            println!(
                "   Added additional {} as {}",
                extra.id,
                extra.path.display()
            );
        }
        for path in &extras.skipped {
            println!("   Skipped existing additional {}", path.display());
        }
    }

    let manifest_ids = if no_file {
        additional.as_slice()
    } else {
        plan.additional_after
            .as_deref()
            .unwrap_or(additional.as_slice())
    };
    match project::update_manifest(&recorded, &ctx.author, &ctx.year, manifest_ids) {
        Ok(files) if !files.is_empty() => {
            println!("   Updated: {}", files.join(", "));
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("   Warning: could not update project manifests: {}", e);
        }
    }

    if update_readme && !no_file {
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
            false,
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
        let config = crate::config::Config::project_path().unwrap();
        fs.write(&config, "[default]\nlicence = \"MIT\"\n").unwrap();
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
            false,
        );
        assert!(
            result.is_ok(),
            "cmd_add proprietary failed: {:?}",
            result.err()
        );
        let saved = fs.read_to_string(&config).unwrap_or_default();
        assert!(saved.contains("proprietary"), "{saved}");
        assert!(
            !saved.to_ascii_lowercase().contains("unlicensed"),
            "{saved}"
        );
        let cargo = fs
            .read_to_string(std::path::Path::new("Cargo.toml"))
            .unwrap();
        assert!(cargo.contains("publish = false"), "{cargo}");
        assert!(cargo.contains("license-file"), "{cargo}");
        assert!(!cargo.contains("UNLICENSED"), "{cargo}");
        assert!(!cargo.contains("proprietary"), "{cargo}");
    }

    #[test]
    fn no_file_proprietary_does_not_reference_a_missing_notice() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\n",
        )
        .unwrap();
        let result = cmd_add(
            Some("proprietary"),
            Some("Acme Corp".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
            None,
            true,
        );
        assert!(result.is_err(), "missing notice must be rejected");
        assert!(!fs.exists(std::path::Path::new("LICENCE.txt")));
        let cargo = fs
            .read_to_string(std::path::Path::new("Cargo.toml"))
            .unwrap();
        assert!(cargo.contains("license = \"MIT\""), "{cargo}");
        assert!(!cargo.contains("license-file"), "{cargo}");
        assert!(!cargo.contains("publish"), "{cargo}");
        assert!(!cargo.contains("UNLICENSED"), "{cargo}");
    }

    #[test]
    fn no_file_proprietary_uses_existing_notice() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\n",
        )
        .unwrap();
        fs.write(std::path::Path::new("LICENCE.txt"), "keep this notice\n")
            .unwrap();
        let result = cmd_add(
            Some("proprietary"),
            Some("Acme Corp".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
            None,
            true,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        assert_eq!(
            fs.read_to_string(std::path::Path::new("LICENCE.txt"))
                .as_deref(),
            Some("keep this notice\n")
        );
        let cargo = fs
            .read_to_string(std::path::Path::new("Cargo.toml"))
            .unwrap();
        assert!(cargo.contains("publish = false"), "{cargo}");
        assert!(cargo.contains("license-file = \"LICENCE.txt\""), "{cargo}");
        assert!(!cargo.contains("UNLICENSED"), "{cargo}");
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
            false,
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
            false,
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
            false,
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
            false,
        );
        assert!(result.is_err());
        assert_eq!(
            fs.read_to_string(std::path::Path::new("LICENCE.txt"))
                .as_deref(),
            Some("keep")
        );
        assert!(!fs.exists(std::path::Path::new("LICENCE.txt.licencify-new")));
    }

    fn with_fs() -> Arc<MemFs> {
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        fs
    }

    #[test]
    fn missing_extra_uses_primary_context_and_actual_extension() {
        let _guard = FsGuard::new();
        let fs = with_fs();
        let config = crate::config::Config::project_path().unwrap();
        let template = config
            .parent()
            .unwrap()
            .join("templates")
            .join("MIT.html.tera");
        fs.create_dir_all(template.parent().unwrap()).unwrap();
        fs.write(&template, "CUSTOM-EXTRA {{ year }} {{ author }}")
            .unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"Apache-2.0\"\nadditional-licences = [\"MIT\"]\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        fs.write(
            Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let result = cmd_add(
            Some("Apache-2.0"),
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Html,
            true,
            false,
            None,
            false,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        let extra = fs
            .read_to_string(Path::new("LICENCE-MIT.html"))
            .expect("LICENCE-MIT.html");
        assert!(extra.contains("CUSTOM-EXTRA"));
        assert!(extra.contains("2024"));
        assert!(extra.contains("Test Author"));
        assert!(!fs.exists(Path::new("LICENCE-MIT.txt")));
        let primary = fs.read_to_string(Path::new("LICENCE.html")).unwrap();
        assert!(primary.contains("Apache"));
        let cargo = fs.read_to_string(Path::new("Cargo.toml")).unwrap();
        assert!(!cargo.contains("license"));
    }

    #[test]
    fn existing_extra_with_another_extension_is_not_rewritten() {
        let _guard = FsGuard::new();
        let fs = with_fs();
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"Apache-2.0\"\nadditional-licences = [\"MIT\"]\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        fs.write(Path::new("LICENCE-MIT.md"), "KEEP-EXTRA").unwrap();
        let result = cmd_add(
            Some("Apache-2.0"),
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
            None,
            false,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE-MIT.md")).as_deref(),
            Some("KEEP-EXTRA")
        );
        assert!(!fs.exists(Path::new("LICENCE-MIT.txt")));
        assert!(fs.exists(Path::new("LICENCE.txt")));
    }

    #[test]
    fn invalid_additional_id_does_not_mutate() {
        let _guard = FsGuard::new();
        let fs = with_fs();
        let config = crate::config::Config::project_path().unwrap();
        fs.write(Path::new("LICENCE.txt"), "keep").unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"MIT\"\nadditional-licences = [\"not-an-id\"]\n",
        )
        .unwrap();
        let result = cmd_add(
            Some("MIT"),
            Some("Test Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            true,
            false,
            None,
            false,
        );
        assert!(result.is_err());
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE.txt")).as_deref(),
            Some("keep")
        );
        assert!(!fs.exists(Path::new("LICENCE.txt.licencify-new")));
    }

    #[test]
    fn unresolved_additional_template_does_not_mutate() {
        let _guard = FsGuard::new();
        let fs = with_fs();
        let config = crate::config::Config::project_path().unwrap();
        fs.write(Path::new("LICENCE.txt"), "keep").unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"MIT\"\nadditional-licences = [\"ISC\"]\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        let template = config.parent().unwrap().join("templates").join("ISC.tera");
        fs.create_dir_all(&template).unwrap();
        let result = cmd_add(
            Some("MIT"),
            Some("Test Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            true,
            false,
            None,
            false,
        );
        assert!(result.is_err(), "unreadable template should fail preflight");
        assert_eq!(
            fs.read_to_string(Path::new("LICENCE.txt")).as_deref(),
            Some("keep")
        );
        assert!(!fs.exists(Path::new("LICENCE-ISC.txt")));
        assert!(!fs.exists(Path::new("LICENCE.txt.licencify-new")));
    }

    #[test]
    fn extra_write_error_is_not_success() {
        let _guard = FsGuard::new();
        let inner = Arc::new(MemFs::new());
        let config = crate::config::Config::project_path().unwrap();
        inner
            .write(
                &config,
                "[default]\nlicence = \"Apache-2.0\"\nadditional-licences = [\"MIT\"]\nlicence_file_name = \"licence\"\n",
            )
            .unwrap();
        crate::fs::set_global_fs(Arc::new(FailExtraWrite(inner.clone())));
        let result = cmd_add(
            Some("Apache-2.0"),
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
            None,
            false,
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("additional licence")
        );
        assert!(inner.exists(Path::new("LICENCE.txt")));
        assert!(!inner.exists(Path::new("LICENCE-MIT.txt")));
    }

    #[test]
    fn no_file_skips_licence_files_and_ambiguous_manifest() {
        let _guard = FsGuard::new();
        let fs = with_fs();
        let config = crate::config::Config::project_path().unwrap();
        fs.write(
            &config,
            "[default]\nlicence = \"MIT\"\nadditional-licences = [\"Apache-2.0\"]\nlicence_file_name = \"licence\"\n",
        )
        .unwrap();
        let cargo = "[package]\nname = \"test\"\nversion = \"0.1.0\"\n";
        fs.write(Path::new("Cargo.toml"), cargo).unwrap();
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
            true,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        assert!(!fs.exists(Path::new("LICENCE.txt")));
        assert!(!fs.exists(Path::new("LICENCE-Apache-2.0.txt")));
        assert_eq!(
            fs.read_to_string(Path::new("Cargo.toml")).as_deref(),
            Some(cargo)
        );
    }

    #[test]
    fn no_file_without_additional_still_updates_manifest() {
        let _guard = FsGuard::new();
        let fs = with_fs();
        fs.write(
            Path::new("Cargo.toml"),
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
            true,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        assert!(!fs.exists(Path::new("LICENCE.txt")));
        let cargo = fs.read_to_string(Path::new("Cargo.toml")).unwrap();
        assert!(cargo.contains("MIT"));
    }

    struct FailExtraWrite(Arc<MemFs>);

    impl Fs for FailExtraWrite {
        fn read_to_string(&self, path: &Path) -> Option<String> {
            self.0.read_to_string(path)
        }

        fn write(&self, path: &Path, contents: &str) -> std::io::Result<()> {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if name.contains("MIT") {
                return Err(std::io::Error::other("extra write failed"));
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
