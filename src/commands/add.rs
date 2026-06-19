use crate::{cli::LicenseFormat, fs::global_fs, project, provider, resolution, template};
use std::io::Write;

pub fn cmd_add(
    spdx: &str,
    author: Option<String>,
    company: Option<String>,
    email: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
    yes: bool,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let config = crate::config::Config::load_effective(None).ok();
    let info = prov.info(spdx)?;
    let ctx =
        resolution::resolve_context(spdx, author, year, company, email, config.as_ref(), &prov)?;

    if !yes {
        println!("About to add license: {} ({})", info.name, info.id);
        println!("  Author:  {}", ctx.author);
        if let Some(ref c) = ctx.company {
            println!("  Company: {}", c);
        }
        if let Some(ref e) = ctx.email {
            println!("  Email:   {}", e);
        }
        println!("  Year:    {}", ctx.year);
        println!("  Format:  {}", format);
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

    let render_ctx = template::render_context(
        &ctx.year,
        &ctx.author,
        ctx.company.as_deref(),
        ctx.email.as_deref(),
    );

    let (content, ext) = match &format {
        LicenseFormat::Html => {
            let html = ctx.resolved.html.as_deref().unwrap_or(&ctx.resolved.text);
            let rendered = template::render_with_context(html, &render_ctx)?;
            (rendered, "html")
        }
        LicenseFormat::Txt => {
            let rendered = template::render_with_context(&ctx.resolved.text, &render_ctx)?;
            (rendered, "txt")
        }
    };

    let filename = ctx.licence_name.file_path(ext);
    let fs = global_fs();

    if fs.exists(&filename) && !yes {
        println!("{} exists. Overwrite? [y/N] ", filename.display());
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    fs.write(&filename, &content)?;
    if spdx.eq_ignore_ascii_case("proprietary") || info.id == "UNLICENSED" {
        println!("✅ Added proprietary notice as {}", filename.display());
    } else {
        println!(
            "✅ Added {} ({}) [from {}] as {}",
            info.name,
            info.id,
            ctx.resolved.source,
            filename.display()
        );
    }

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
