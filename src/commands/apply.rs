use crate::{cli::LicenseFormat, fs::global_fs, provider, resolution};
use anyhow::Context;
use std::io::{IsTerminal, Write};

pub fn cmd_apply(
    ids: &[String],
    root: bool,
    yes: bool,
    update_readme: Option<bool>,
) -> anyhow::Result<()> {
    let config = crate::config::Config::load_effective(None)?;
    let update_readme =
        crate::readme::readme_updates_enabled(update_readme, config.default.update_readme);
    let provider = provider::LicenseProvider::load()?;
    let selected = ids
        .iter()
        .map(|id| provider.info(id).map(|info| info.id))
        .collect::<anyhow::Result<Vec<_>>>()?;
    for (index, id) in selected.iter().enumerate() {
        anyhow::ensure!(
            !selected[..index]
                .iter()
                .any(|previous| previous.eq_ignore_ascii_case(id)),
            "Repeated licence id: {id}"
        );
    }
    let has_proprietary = selected.iter().any(|id| id == "proprietary");
    anyhow::ensure!(
        !has_proprietary || selected.len() == 1,
        "proprietary cannot be combined with another licence id"
    );
    let spdx = super::generate::selected_licence_id(
        selected.first().map(String::as_str),
        config.default.license.as_deref(),
        false,
    )?;
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

    let additional = if selected.len() > 1 {
        selected[1..].to_vec()
    } else {
        config
            .default
            .additional_licences
            .clone()
            .unwrap_or_default()
    };
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
    if !selected.is_empty() {
        crate::config::Config::save_applied_selection(
            &spdx,
            &context.author,
            &format.to_string(),
            (selected.len() > 1).then_some(&additional),
            root,
        )?;
    }

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
    if update_readme {
        crate::readme::report_readme(crate::readme::update_readme(&info.id, &primary.path));
    }
    Ok(())
}
