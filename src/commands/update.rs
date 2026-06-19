use crate::{cli::LicenseFormat, fs::global_fs, project, provider, resolution, template};

pub fn cmd_update(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = crate::config::Config::load_effective(None).ok();
    let info = prov.info(spdx)?;
    let ctx = resolution::resolve_context(spdx, author, year, config.as_ref(), &prov)?;

    let (content, ext) = match &format {
        LicenseFormat::Html => {
            let html = ctx.resolved.html.as_deref().unwrap_or(&ctx.resolved.text);
            let rendered = template::render(html, &ctx.year, &ctx.author)?;
            (rendered, "html")
        }
        LicenseFormat::Txt => {
            let rendered = template::render(&ctx.resolved.text, &ctx.year, &ctx.author)?;
            (rendered, "txt")
        }
    };

    let filename = ctx.licence_name.file_path(ext);
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
        ctx.resolved.source,
        filename.display()
    );

    // Update project config defaults if a project config exists
    let fmt_str = format.to_string();
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

    Ok(())
}
