use crate::{
    cli::CacheAction,
    fs::{Fs, global_fs},
    provider, spdx,
};
use anyhow::Context;
use std::{io::Write, path::Path};

fn cache_size(fs: &dyn Fs, dir: &Path) -> u64 {
    // SPDX details are stored as flat JSON files directly in this directory.
    fs.read_dir(dir)
        .iter()
        .filter_map(|path| fs.read_to_string(path))
        .map(|contents| contents.len() as u64)
        .sum()
}

pub fn cmd_cache(action: Option<CacheAction>) -> anyhow::Result<()> {
    match action {
        None | Some(CacheAction::Info) => cmd_cache_info(),
        Some(CacheAction::Clear) => cmd_cache_clear(),
        Some(CacheAction::Update) => cmd_cache_update(),
        Some(CacheAction::Fetch { id }) => cmd_cache_fetch(id.as_deref()),
        Some(CacheAction::FetchAll) => cmd_cache_fetch(None),
    }
}

fn cmd_cache_info() -> anyhow::Result<()> {
    let dir = provider::spdx_cache_dir()?;
    let fs = global_fs();

    let count = if fs.exists(&dir) {
        fs.read_dir(&dir).len()
    } else {
        0
    };
    println!("Cache directory: {}", dir.display());
    println!("Cached files: {}", count);
    println!("Cache size: {} bytes", cache_size(fs.as_ref(), &dir));
    Ok(())
}

fn cmd_cache_clear() -> anyhow::Result<()> {
    let dir = provider::spdx_cache_dir()?;
    let fs = global_fs();

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

fn cmd_cache_update() -> anyhow::Result<()> {
    let dir = provider::spdx_cache_dir()?;
    let fs = global_fs();

    let url = format!("{}/licenses.json", provider::spdx_base_url());
    let mut resp = ureq::get(&url)
        .call()
        .context("Failed to fetch SPDX licence index")?;
    let body = resp
        .body_mut()
        .read_to_string()
        .context("Failed to read SPDX licenses.json response")?;

    let index: spdx::SpdxIndex =
        serde_json::from_str(&body).context("Failed to parse SPDX licenses.json response")?;

    if !fs.exists(&dir) {
        fs.create_dir_all(&dir)
            .context("Failed to create SPDX cache directory")?;
    }

    let index_path = dir.join("licenses.json");
    fs.write(&index_path, &body)
        .context("Failed to write SPDX licenses.json cache")?;

    println!(
        "Updated SPDX licence index ({} licences)",
        index.licenses.len()
    );
    Ok(())
}

fn cmd_cache_fetch(id: Option<&str>) -> anyhow::Result<()> {
    let dir = provider::spdx_cache_dir()?;
    let prov = provider::LicenseProvider::with_spdx_cache(&dir)?;

    match id {
        Some(license_id) => {
            if prov.get_cached(license_id).is_some() {
                println!("{license_id} is already cached");
                return Ok(());
            }
            prov.fetch_detail(license_id)?;
            println!("Fetched {license_id}");
            Ok(())
        }
        None => {
            let index = spdx::SpdxIndex::load_with_cache(&dir)?;
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

            if fetched > 0 || failed > 0 {
                print!("\r{:width$}\r", "", width = 60);
                std::io::stdout().flush().ok();
            }

            println!(
                "Done. {} fetched, {} skipped (already cached), {} failed out of {} total",
                fetched, skipped, failed, total
            );
            Ok(())
        }
    }
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
        let result = cmd_cache(Some(CacheAction::Info));
        assert!(result.is_ok());

        let bare = cmd_cache(None);
        assert!(bare.is_ok());
    }

    #[test]
    fn cmd_cache_clear_removes_only_spdx_cache() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        let dir = provider::spdx_cache_dir().unwrap();
        let parent = dir.parent().unwrap();
        fs.create_dir_all(&dir).unwrap();
        fs.write_file(dir.join("MIT.json"), "{}");
        fs.write_file(parent.join("settings.json"), "{}");
        assert_eq!(cache_size(fs.as_ref(), &dir), 2);
        crate::fs::set_global_fs(fs.clone());

        cmd_cache(Some(CacheAction::Clear)).unwrap();

        assert!(!fs.exists(&dir.join("MIT.json")));
        assert!(fs.exists(&parent.join("settings.json")));
    }

    #[test]
    fn cmd_cache_fetch_skips_already_cached() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        let dir = provider::spdx_cache_dir().unwrap();
        fs.create_dir_all(&dir).unwrap();
        fs.write_file(
            dir.join("MIT.json"),
            r#"{"licenseId":"MIT","name":"MIT License","licenseText":"cached"}"#,
        );
        crate::fs::set_global_fs(fs.clone());

        let result = cmd_cache(Some(CacheAction::Fetch {
            id: Some("MIT".to_string()),
        }));
        assert!(result.is_ok());
        assert_eq!(
            fs.read_to_string(&dir.join("MIT.json")).unwrap(),
            r#"{"licenseId":"MIT","name":"MIT License","licenseText":"cached"}"#
        );
    }

    #[test]
    fn cmd_cache_fetch_fails_on_unknown_id() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs);

        let result = cmd_cache(Some(CacheAction::Fetch {
            id: Some("UNKNOWN-NEVER-EXISTS".to_string()),
        }));
        assert!(result.is_err());
    }
}
