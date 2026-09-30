use crate::{cli::LicenseFormat, fs::global_fs, project, provider, resolution};
use std::io::Write;

pub fn cmd_add(
    spdx: &str,
    author: Option<String>,
    company: Option<String>,
    email: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
    yes: bool,
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

    if !yes {
        println!("About to add license: {} ({})", info.name, info.id);
        println!("  Author:  {}", ctx.author);
        if let Some(ref c) = ctx.company {
            println!("  Company: {}", c);
        }
        if let Some(ref e) = ctx.email {
            println!("  Email:   {}", e);
        }
        println!("  Format:  {}", ctx.resolved.format);
        println!();
        print!("Continue? [Y/n] ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let rendered = super::generate::render_primary(&ctx)?;
    let fs = global_fs();

    if fs.exists(&rendered.path) && !yes {
        println!("{} exists. Overwrite? [y/N] ", rendered.path.display());
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    super::generate::write_primary(&rendered)?;
    if spdx.eq_ignore_ascii_case("proprietary") || info.id == "UNLICENSED" {
        println!("✅ Added proprietary notice as {}", rendered.path.display());
    } else {
        println!(
            "✅ Added {} ({}) [from {}] as {}",
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
        let result = cmd_add(
            "MIT",
            Some("Test Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
            true,
            false,
        );
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
            "proprietary",
            Some("Acme Corp".into()),
            Some("Acme Corp".into()),
            Some("legal@acme.com".into()),
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
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
            "MIT",
            Some("Test Author".into()),
            None,
            None,
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
            false,
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());
        let text = fs
            .read_to_string(std::path::Path::new("LICENCE.txt"))
            .expect("LICENCE.txt");
        assert!(text.contains("MIT License"));
        assert!(text.contains("Copyright (c) 2024 Test Author"));
    }
}
