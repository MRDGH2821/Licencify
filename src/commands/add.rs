use crate::{cli::LicenseFormat, config::Config, project, provider, resolution, template};
use std::io::Write;

pub fn cmd_add(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
    yes: bool,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = Config::load().ok();
    let info = prov.info(spdx)?;
    let author = resolution::resolve_author(author)?;
    let year = resolution::resolve_year(year);
    let base_name = config
        .as_ref()
        .map(|c| c.licence_name())
        .unwrap_or_else(|| "LICENCE".to_string());

    if !yes {
        println!("About to add license: {} ({})", info.name, info.id);
        println!("  Author: {}", author);
        println!("  Year:   {}", year);
        println!("  Format: {}", format);
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

    let resolved = resolution::resolve_template(&info.id)?;

    let (content, ext) = match &format {
        LicenseFormat::Html => {
            let html = resolved.html.as_deref().unwrap_or(&resolved.text);
            let rendered = template::render(html, &year, &author)?;
            (rendered, "html")
        }
        LicenseFormat::Txt => {
            let rendered = template::render(&resolved.text, &year, &author)?;
            (rendered, "txt")
        }
    };

    let filename = format!("{}.{}", base_name, ext);

    if std::path::Path::new(&filename).exists() && !yes {
        println!("{} exists. Overwrite? [y/N] ", filename);
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    std::fs::write(&filename, &content)?;
    println!(
        "✅ Added {} ({}) [from {}] as {}",
        info.name, info.id, resolved.source, filename
    );

    match project::update_manifest(&info.id, &author, &year) {
        Ok(files) if !files.is_empty() => {
            println!("   Updated: {}", files.join(", "));
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("   Warning: could not update project manifests: {}", e);
        }
    }

    Ok(())
}
