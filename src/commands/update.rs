use crate::{cli::LicenseFormat, fs::global_fs, project, provider, resolution};

pub fn cmd_update(
    spdx: &str,
    author: Option<String>,
    company: Option<String>,
    email: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
    update_readme: bool,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = crate::config::Config::load_effective(None)?;
    let update_readme = update_readme || config.default.update_readme.unwrap_or(false);
    let info = prov.info(spdx)?;
    let ctx = resolution::resolve_context(
        spdx,
        author,
        year,
        company,
        email,
        Some(&config),
        &prov,
        &format,
    )?;

    let rendered = super::generate::render_primary(&ctx)?;
    let fs = global_fs();

    if !fs.exists(&rendered.path) {
        anyhow::bail!(
            "{} not found. Use `licencify add {}` to create it first.",
            rendered.path.display(),
            spdx
        );
    }

    super::generate::write_primary(&rendered)?;
    if spdx.eq_ignore_ascii_case("proprietary") || info.id == "UNLICENSED" {
        println!(
            "✅ Updated proprietary notice as {}",
            rendered.path.display()
        );
    } else {
        println!(
            "✅ Updated {} ({}) [from {}] as {}",
            info.name,
            info.id,
            ctx.resolved.source,
            rendered.path.display()
        );
    }

    // Update project config defaults if a project config exists
    let fmt_str = ctx.resolved.format.to_string();
    match crate::config::Config::update_project_defaults(&info.id, &ctx.author, &fmt_str) {
        Ok(true) => println!("   Updated project config defaults"),
        Ok(false) => {}
        Err(e) => {
            eprintln!("   Warning: could not update project config: {}", e);
        }
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

    // Update README with license badge if requested
    if update_readme {
        match crate::readme::update_readme(&info.id) {
            Ok(true) => {}
            Ok(false) => {
                println!("   README: not found or already has license section");
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
    use crate::commands::add::cmd_add;
    use crate::fs::{FsGuard, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_update_requires_existing_file() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        let result = cmd_update(
            "MIT",
            Some("Test Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn cmd_update_replaces_content() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let add_result = cmd_add(
            "MIT",
            Some("Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            true,
            false,
        );
        assert!(add_result.is_ok());
        let result = cmd_update(
            "Apache-2.0",
            Some("Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            false,
        );
        assert!(result.is_ok(), "cmd_update failed: {:?}", result.err());
    }

    #[test]
    fn cmd_update_writes_expected_mit_text() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs.write(std::path::Path::new("LICENCE.txt"), "previous licence text")
            .unwrap();
        let result = cmd_update(
            "MIT",
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            false,
        );
        assert!(result.is_ok(), "cmd_update failed: {:?}", result.err());
        let text = fs
            .read_to_string(std::path::Path::new("LICENCE.txt"))
            .expect("LICENCE.txt");
        assert!(text.contains("MIT License"));
        assert!(text.contains("Copyright (c) 2024 Test Author"));
        assert!(!text.contains("previous licence text"));
    }
}
