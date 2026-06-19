use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};

/// Abstraction over filesystem operations, enabling testing without real I/O.
pub trait Fs: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Option<String>;
    fn write(&self, path: &Path, contents: &str) -> std::io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    /// List direct children of a directory (file and dir names).
    fn read_dir(&self, path: &Path) -> Vec<PathBuf>;
    /// Remove a directory and all its contents recursively.
    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()>;
}

/// Real filesystem — delegates to `std::fs`.
pub struct RealFs;

impl Fs for RealFs {
    fn read_to_string(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn write(&self, path: &Path, contents: &str) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(path)
            .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.path()).collect())
            .unwrap_or_default()
    }

    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }
}

/// In-memory filesystem for testing.
pub struct MemFs {
    files: RwLock<HashMap<PathBuf, String>>,
    dirs: RwLock<HashMap<PathBuf, bool>>,
}

impl MemFs {
    pub fn new() -> Self {
        let mut dirs = HashMap::new();
        dirs.insert(PathBuf::from("/"), true);
        Self {
            files: RwLock::new(HashMap::new()),
            dirs: RwLock::new(dirs),
        }
    }

    pub fn write_file(&self, path: impl Into<PathBuf>, contents: &str) {
        self.files
            .write()
            .unwrap()
            .insert(path.into(), contents.to_string());
    }

    pub fn create_dir(&self, path: impl Into<PathBuf>) {
        self.dirs.write().unwrap().insert(path.into(), true);
    }
}

impl Default for MemFs {
    fn default() -> Self {
        Self::new()
    }
}

impl Fs for MemFs {
    fn read_to_string(&self, path: &Path) -> Option<String> {
        self.files.read().unwrap().get(path).cloned()
    }

    fn write(&self, path: &Path, contents: &str) -> std::io::Result<()> {
        self.files
            .write()
            .unwrap()
            .insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.read().unwrap().contains_key(path)
            || self.dirs.read().unwrap().contains_key(path)
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.dirs.write().unwrap().insert(path.to_path_buf(), true);
        Ok(())
    }

    fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
        let prefix = format!("{}/", path.display());
        let files = self.files.read().unwrap();
        let mut entries: Vec<PathBuf> = files
            .keys()
            .filter(|k| k.to_string_lossy().starts_with(&prefix))
            .map(|k| k.to_path_buf())
            .collect();
        let dirs = self.dirs.read().unwrap();
        for k in dirs.keys() {
            if k != path && k.to_string_lossy().starts_with(&prefix) {
                entries.push(k.to_path_buf());
            }
        }
        entries
    }

    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
        let prefix = format!("{}/", path.display());
        self.files
            .write()
            .unwrap()
            .retain(|k, _| !k.to_string_lossy().starts_with(&prefix));
        self.dirs.write().unwrap().remove(path);
        Ok(())
    }
}

/// Global filesystem handle, swapable for testing.
static GLOBAL_FS: LazyLock<RwLock<Arc<dyn Fs>>> = LazyLock::new(|| RwLock::new(Arc::new(RealFs)));

/// Set the global filesystem (for testing).
pub fn set_global_fs(fs: Arc<dyn Fs>) {
    *GLOBAL_FS.write().unwrap() = fs;
}

/// Reset to real filesystem.
pub fn reset_global_fs() {
    *GLOBAL_FS.write().unwrap() = Arc::new(RealFs);
}

/// Access the global filesystem.
pub fn global_fs() -> Arc<dyn Fs> {
    GLOBAL_FS.read().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memfs_basic_ops() {
        let fs = MemFs::new();
        let path = Path::new("/test.txt");

        assert!(!fs.exists(path));
        fs.write(path, "hello").unwrap();
        assert!(fs.exists(path));
        assert_eq!(fs.read_to_string(path).as_deref(), Some("hello"));

        fs.write(path, "world").unwrap();
        assert_eq!(fs.read_to_string(path).as_deref(), Some("world"));
    }

    #[test]
    fn memfs_create_dir() {
        let fs = MemFs::new();
        let dir = Path::new("/some/dir");

        assert!(!fs.exists(dir));
        fs.create_dir_all(dir).unwrap();
        assert!(fs.exists(dir));
    }

    #[test]
    fn memfs_read_dir() {
        let fs = MemFs::new();
        fs.create_dir_all(Path::new("/project")).unwrap();
        fs.write_file("/project/a.txt", "a");
        fs.write_file("/project/b.txt", "b");
        fs.write_file("/other/c.txt", "c");

        let entries = fs.read_dir(Path::new("/project"));
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn memfs_remove_dir_all() {
        let fs = MemFs::new();
        fs.create_dir_all(Path::new("/project")).unwrap();
        fs.write_file("/project/a.txt", "a");
        fs.write_file("/project/b.txt", "b");

        assert!(fs.exists(Path::new("/project/a.txt")));
        fs.remove_dir_all(Path::new("/project")).unwrap();
        assert!(!fs.exists(Path::new("/project/a.txt")));
        assert!(!fs.exists(Path::new("/project")));
    }
}
