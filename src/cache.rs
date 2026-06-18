use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct LicenseCache {
    cache_dir: PathBuf,
}

impl LicenseCache {
    /// Create a new cache pointing at `<XDG_CACHE_HOME>/licencify/templates/`.
    /// The directory is created if it does not already exist.
    pub fn new() -> Result<Self> {
        let base = dirs::cache_dir().context("unable to determine XDG cache directory")?;
        let cache_dir = base.join("licencify").join("templates");
        fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;
        Ok(Self { cache_dir })
    }

    /// Return the file path where a license with the given SPDX key would be cached.
    pub fn path_for(&self, spdx_key: &str) -> PathBuf {
        self.cache_dir.join(format!("{spdx_key}.json"))
    }

    /// Retrieve the cached content for `spdx_key`, or `None` if not present.
    pub fn get(&self, spdx_key: &str) -> Option<String> {
        let path = self.path_for(spdx_key);
        fs::read_to_string(path).ok()
    }

    /// Store `content` in the cache under `spdx_key`.
    pub fn put(&self, spdx_key: &str, content: &str) -> Result<()> {
        let path = self.path_for(spdx_key);
        fs::write(&path, content)
            .with_context(|| format!("failed to write cache file for {spdx_key}"))?;
        Ok(())
    }

    /// Remove all cached files. Returns the number of files removed.
    pub fn clear(&self) -> Result<usize> {
        let mut count = 0usize;
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir).context("failed to read cache directory")? {
                let entry = entry.context("failed to read cache entry")?;
                let path = entry.path();
                if path.is_file() {
                    fs::remove_file(&path)
                        .with_context(|| format!("failed to remove {}", path.display()))?;
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    /// Return a reference to the cache directory path.
    pub fn dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Return the number of files currently in the cache.
    pub fn count(&self) -> usize {
        fs::read_dir(&self.cache_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .count()
            })
            .unwrap_or(0)
    }
}
