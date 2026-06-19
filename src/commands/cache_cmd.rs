use crate::{cli::CacheAction, fs::global_fs, provider, spdx};
use std::io::Write;

fn api_dir() -> anyhow::Result<std::path::PathBuf> {
    Ok(dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("unable to determine XDG cache directory"))?
        .join("licencify")
        .join("api"))
}

pub fn cmd_cache(action: CacheAction) -> anyhow::Result<()> {
    let dir = api_dir()?;
    let fs = global_fs();

    match action {
        CacheAction::Clear => {
            let count = if fs.exists(&dir) {
                let n = fs.read_dir(&dir).len();
                fs.remove_dir_all(&dir).ok();
                n
            } else {
                0
            };
            println!(
                "Cleared {} cached API responses from {}",
                count,
                dir.display()
            );
            Ok(())
        }
        CacheAction::Info => {
            let count = if fs.exists(&dir) {
                fs.read_dir(&dir).len()
            } else {
                0
            };
            println!("Cache directory: {}", dir.display());
            println!("Cached responses: {}", count);
            Ok(())
        }
        CacheAction::FetchAll => cmd_cache_fetch_all(),
    }
}

fn cmd_cache_fetch_all() -> anyhow::Result<()> {
    let index = spdx::SpdxIndex::load()?;
    let dir = api_dir()?;
    let prov = provider::LicenseProvider::with_api_cache(&dir)?;

    let total = index.licenses.len();
    let mut fetched = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for (i, license) in index.licenses.iter().enumerate() {
        let id = &license.license_id;

        if prov.get_cached(id).is_some() {
            skipped += 1;
            continue;
        }

        print!("\r[{}/{}] Fetching {}...", i + 1, total, id);
        std::io::stdout().flush().ok();

        match prov.fetch_detail(id) {
            Ok(_) => {
                fetched += 1;
            }
            Err(e) => {
                eprintln!("\n  ⚠ Failed to cache {}: {}", id, e);
                failed += 1;
            }
        }
    }

    print!("\r{:width$}\r", "", width = 60);
    std::io::stdout().flush().ok();

    println!(
        "Done. {} fetched, {} skipped (already cached), {} failed out of {} total",
        fetched, skipped, failed, total
    );
    Ok(())
}
