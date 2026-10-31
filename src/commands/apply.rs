use crate::{cli::LicenseFormat, fs::global_fs, provider, resolution};
use anyhow::Context;

pub fn cmd_apply() -> anyhow::Result<()> {
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

    let additional = config
        .default
        .additional_licences
        .clone()
        .unwrap_or_default();
    let primary = super::generate::render_primary(&context)?;
    let extras =
        super::generate::plan_missing_extras(&context, &config, &provider, &additional, &format)?;
    let fs = global_fs();
    fs.create_new(&primary.path, &primary.content)
        .with_context(|| format!("Failed to write licence file {}", primary.path.display()))?;
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
