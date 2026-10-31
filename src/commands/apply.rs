use crate::{cli::LicenseFormat, fs::global_fs, provider, resolution};
use anyhow::Context;
use std::io::{IsTerminal, Write};

pub fn cmd_apply(yes: bool) -> anyhow::Result<()> {
    let config = crate::config::Config::load_effective(None)?;
    let spdx =
        super::generate::selected_licence_id(None, config.default.license.as_deref(), false)?;
    let provider = provider::LicenseProvider::load()?;
    let info = provider.info(&spdx)?;
    let format = match config
        .default
        .format
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html") => LicenseFormat::Html,
        Some("md") => LicenseFormat::Md,
        _ => LicenseFormat::Txt,
    };
    let context = resolution::resolve_context(
        &spdx,
        None,
        None,
        None,
        None,
        Some(&config),
        &provider,
        &format,
    )?;
    if config
        .default
        .author
        .as_deref()
        .is_none_or(|author| author.trim().is_empty())
    {
        println!("Using author from git config user.name");
    }
    if config
        .default
        .email
        .as_deref()
        .is_none_or(|email| email.trim().is_empty())
        && context.email.is_some()
    {
        println!("Using email from git config user.email");
    }

    let fs = global_fs();
    let existing_primary = super::generate::detect_primary(&*fs)?;

    if let Some(existing) = &existing_primary {
        if !yes {
            if !std::io::stdin().is_terminal() {
                anyhow::bail!(
                    "Refusing to replace existing primary licence file {}: run interactively or pass --yes",
                    existing.display()
                );
            }
            let prompt = format!(
                "About to replace the primary licence with {} ({}). Continue? [y/N] ",
                info.name, info.id
            );
            print!("{prompt}");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let answer = input.trim();
            let confirmed = answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes");
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    let additional = config
        .default
        .additional_licences
        .clone()
        .unwrap_or_default();
    let primary = super::generate::render_primary(&context)?;
    let extras =
        super::generate::plan_missing_extras(&context, &config, &provider, &additional, &format)?;

    let stage = super::generate::staging_path(&primary.path);
    anyhow::ensure!(
        !fs.exists(&stage),
        "Refusing to overwrite staging file {}",
        stage.display()
    );
    fs.write(&stage, &primary.content)
        .with_context(|| format!("Failed to stage replacement {}", stage.display()))?;

    if let Err(error) = fs.rename(&stage, &primary.path) {
        return Err(anyhow::Error::from(error).context(format!(
            "Replacement remains staged at {} and the existing primary was left in place",
            stage.display()
        )));
    }

    if let Some(existing) = &existing_primary {
        if existing != &primary.path {
            if let Err(error) = fs.remove_file(existing) {
                return Err(anyhow::Error::from(error).context(format!(
                    "Replacement is at {} and the previous primary {} was left in place",
                    primary.path.display(),
                    existing.display()
                )));
            }
        }
    }

    super::generate::write_missing_extras(&extras.missing)?;

    println!(
        "✅ Applied {} ({}) as {}",
        info.name,
        info.id,
        primary.path.display()
    );
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
    Ok(())
}
