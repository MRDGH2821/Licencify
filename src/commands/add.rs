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
    update_readme: bool,
    no_file: bool,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = crate::config::Config::load_effective(None)?;
    let update_readme = update_readme || config.default.update_readme.unwrap_or(false);
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
    if no_file {
        println!("Skipped licence file writes (--no-file).");
    } else {
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

        let saved = super::generate::commit_primary(&plan, &recorded, &ctx.author)?;
        if recorded == "proprietary" {
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
    }

    match project::update_manifest(&recorded, &ctx.author, &ctx.year) {
        Ok(files) if !files.is_empty() => {
            println!("   Updated: {}", files.join(", "));
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("   Warning: could not update project manifests: {}", e);
        }
    }

    if update_readme && !no_file {
        match crate::readme::update_readme(&info.id) {
            Ok(true) => {}
            Ok(false) => {
                if !yes {
                    println!("   README: not found or already has license section");
                }
            }
            Err(e) => {
                eprintln!("   Warning: could not update README: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LicenseFormat;
    use crate::fs::{FsGuard, MemFs};
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
            false,
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
            false,
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
            false,
            true,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
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
            false,
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
            false,
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
            false,
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
            false,
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
            false,
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
}
