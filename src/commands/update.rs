use crate::{project, provider, resolution, template};

pub fn cmd_update(spdx: &str, author: Option<String>, year: Option<String>) -> anyhow::Result<()> {
    let prov = provider::LicenseProvider::load()?;
    let info = prov.info(spdx)?;
    let author = resolution::resolve_author(author)?;
    let year = resolution::resolve_year(year);

    let (raw_text, source) = resolution::resolve_template(&info.id)?;
    let content = template::render(&raw_text, &year, &author)?;

    let filename = if info.id.to_uppercase() == info.id {
        format!("LICENSE-{}", info.id)
    } else {
        "LICENSE".to_string()
    };

    std::fs::write(&filename, &content)?;
    println!("✅ Updated {} ({}) [from {}]", info.name, info.id, source);

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
