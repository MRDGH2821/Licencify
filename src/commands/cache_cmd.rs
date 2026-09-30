use crate::{
    cli::CacheAction,
    fs::{Fs, global_fs},
    provider, spdx,
};
use std::{io::Write, path::Path};

fn cache_size(fs: &dyn Fs, dir: &Path) -> u64 {
    // SPDX details are stored as flat JSON files directly in this directory.
    fs.read_dir(dir)
        .iter()
        .filter_map(|path| fs.read_to_string(path))
        .map(|contents| contents.len() as u64)
        .sum()
}

pub fn cmd_cache(action: CacheAction) -> anyhow::Result<()> {
    let dir = provider::spdx_cache_dir()?;
    let fs = global_fs();

    match action {
        CacheAction::Clear => {
            let count = if fs.exists(&dir) {
                let n = fs.read_dir(&dir).len();
                fs.remove_dir_all(&dir)?;
                n
            } else {
                0
            };
            println!(
                "Cleared {} cached SPDX responses from {}",
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
            println!("Cache size: {} bytes", cache_size(fs.as_ref(), &dir));
            Ok(())
        }
        CacheAction::FetchAll => cmd_cache_fetch_all(),
    }
}

fn cmd_cache_fetch_all() -> anyhow::Result<()> {
    let index = spdx::SpdxIndex::load()?;
    let dir = provider::spdx_cache_dir()?;
    let prov = provider::LicenseProvider::with_spdx_cache(&dir)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CacheAction;
    use crate::fs::{FsGuard, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_cache_info_with_empty_cache() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        let result = cmd_cache(CacheAction::Info);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_cache_clear_removes_only_spdx_cache() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        let dir = provider::spdx_cache_dir().unwrap();
        let parent = dir.parent().unwrap();
        fs.create_dir_all(&dir);
        fs.write_file(dir.join("MIT.json"), "{}");
        fs.write_file(parent.join("settings.json"), "{}");
        assert_eq!(cache_size(fs.as_ref(), &dir), 2);
        crate::fs::set_global_fs(fs.clone());

        cmd_cache(CacheAction::Clear).unwrap();

        assert!(!fs.exists(&dir.join("MIT.json")));
        assert!(fs.exists(&parent.join("settings.json")));
    }
}
