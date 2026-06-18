use crate::{cli::LicenseFormat, config::Config, project, provider, resolution, template};

pub fn cmd_update(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = Config::load_effective().ok();
    let info = prov.info(spdx)?;
    let author = resolution::resolve_author(author)?;
    let year = resolution::resolve_year(year);
    let base_name = config
        .as_ref()
        .map(|c| c.licence_name())
        .unwrap_or_else(|| "LICENCE".to_string());

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

    if !std::path::Path::new(&filename).exists() {
        anyhow::bail!(
            "{} not found. Use `licencify add {}` to create it first.",
            filename,
            spdx
        );
    }

    std::fs::write(&filename, &content)?;
    println!(
        "✅ Updated {} ({}) [from {}] as {}",
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
