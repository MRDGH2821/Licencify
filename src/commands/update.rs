use crate::{
    cli::LicenseFormat, config::Config, fs::global_fs, licence_name::LicenceName, project,
    provider, resolution, template,
};

pub fn cmd_update(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = Config::load_effective().ok();
    let info = prov.info(spdx)?;
    let author = resolution::resolve_author(author, config.as_ref())?;
    let year = resolution::resolve_year(year, config.as_ref());
    let licence_name = LicenceName::resolve(
        config
            .as_ref()
            .and_then(|c| c.default.licence_name.as_deref()),
    );

    let resolved = resolution::resolve_template(&info.id, config.as_ref())?;

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

    let filename = licence_name.file_path(ext);
    let fs = global_fs();

    if !fs.exists(&filename) {
        anyhow::bail!(
            "{} not found. Use `licencify add {}` to create it first.",
            filename.display(),
            spdx
        );
    }

    fs.write(&filename, &content)?;
    println!(
        "✅ Updated {} ({}) [from {}] as {}",
        info.name,
        info.id,
        resolved.source,
        filename.display()
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
