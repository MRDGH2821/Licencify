use crate::{cache, cli::CacheAction, provider, spdx};
use std::io::Write;

pub fn cmd_cache(action: CacheAction) -> anyhow::Result<()> {
    let cache = cache::LicenseCache::new()?;
    let api_dir = cache.dir().parent().unwrap().join("api");

    match action {
        CacheAction::Clear => {
            let count = cache.clear()?;
            let api_count = if api_dir.exists() {
                let n = std::fs::read_dir(&api_dir)
                    .map(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .filter(|e| e.path().is_file())
                            .count()
                    })
                    .unwrap_or(0);
                std::fs::remove_dir_all(&api_dir).ok();
                n
            } else {
                0
            };
            println!(
                "Cleared {} cached templates from {}",
                count,
                cache.dir().display()
            );
            if api_count > 0 {
                println!(
                    "Cleared {} cached API responses from {}",
                    api_count,
                    api_dir.display()
                );
            }
            Ok(())
        }
        CacheAction::Info => {
            println!("Cache directory: {}", cache.dir().display());
            println!("Cached templates: {}", cache.count());
            let api_count = if api_dir.exists() {
                std::fs::read_dir(&api_dir)
                    .map(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .filter(|e| e.path().is_file())
                            .count()
                    })
                    .unwrap_or(0)
            } else {
                0
            };
            println!("API responses:    {}", api_count);
            Ok(())
        }
        CacheAction::FetchAll => cmd_cache_fetch_all(&cache),
    }
}

fn cmd_cache_fetch_all(cache: &cache::LicenseCache) -> anyhow::Result<()> {
    let index = spdx::SpdxIndex::load()?;
    let registry_dir = cache.dir().parent().unwrap().join("api");
    let prov = provider::LicenseProvider::with_api_cache(&registry_dir)?;

    let total = index.licenses.len();
    let mut cached = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for (i, license) in index.licenses.iter().enumerate() {
        let id = &license.license_id;
        let lower = id.to_lowercase();

        if cache.get(&lower).is_some() {
            skipped += 1;
            continue;
        }

        print!("\r[{}/{}] Fetching {}...", i + 1, total, id);
        std::io::stdout().flush().ok();

        match prov.fetch_detail(id) {
            Ok(detail) => {
                let _ = cache.put(&lower, &detail.license_text);
                cached += 1;
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
        "Done. {} cached, {} skipped (already cached), {} failed out of {} total",
        cached, skipped, failed, total
    );
    Ok(())
}
