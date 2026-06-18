use crate::{project, provider, resolution, template};

pub fn cmd_add(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    yes: bool,
) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let info = prov.info(spdx)?;
    let author = resolution::resolve_author(author)?;
    let year = resolution::resolve_year(year);

    if !yes {
        println!("About to add license: {} ({})", info.name, info.id);
        println!("  Author: {}", author);
        println!("  Year:   {}", year);
        println!();
        print!("Continue? [Y/n] ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let (raw_text, source) = resolution::resolve_template(&info.id)?;
    let content = template::render(&raw_text, &year, &author)?;

    let filename = if info.id.to_uppercase() == info.id {
        format!("LICENSE-{}", info.id)
    } else {
        "LICENSE".to_string()
    };

    if std::path::Path::new(&filename).exists() && !yes {
        println!("{} exists. Overwrite? [y/N] ", filename);
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    std::fs::write(&filename, &content)?;
    println!("✅ Added {} ({}) [from {}]", info.name, info.id, source);

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
