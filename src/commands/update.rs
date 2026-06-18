use crate::{cli::LicenseFormat, project, provider, resolution, template};

pub fn cmd_update(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let info = prov.info(spdx)?;
    let author = resolution::resolve_author(author)?;
    let year = resolution::resolve_year(year);

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

    let filename = format!("LICENCE.{}", ext);

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
