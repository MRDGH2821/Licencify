use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, RwLock};

/// Serializes tests that use the global filesystem.
///
/// Rust runs tests in parallel by default. Without this, two tests calling
/// `set_global_fs()` / `reset_global_fs()` concurrently race on the RwLock.
#[doc(hidden)]
pub static GLOBAL_FS_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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

/// Check if `child` is a direct child of `parent`.
/// Returns the child's name if it is, None otherwise.
/// Assumes `full_path` starts with `prefix` (caller must check).
fn direct_child_name<'a>(full_path: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = &full_path[prefix.len()..];
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(rest)
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
        // Create all ancestor directories, then the target
        let mut ancestors: Vec<PathBuf> = path.ancestors().map(Path::to_path_buf).collect();
        ancestors.reverse(); // shallowest first
        let mut dirs = self.dirs.write().unwrap();
        for ancestor in ancestors {
            dirs.insert(ancestor, true);
        }
        Ok(())
    }

    fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
        let prefix = format!("{}/", path.display());
        let mut entries = Vec::new();

        // Collect direct children from files
        {
            let files = self.files.read().unwrap();
            for k in files.keys() {
                let key_str = k.to_string_lossy();
                if key_str.starts_with(&prefix) {
                    if let Some(name) = direct_child_name(&key_str, &prefix) {
                        let mut child = path.to_path_buf();
                        child.push(name);
                        entries.push(child);
                    }
                }
            }
        }

        // Collect direct children from dirs (excluding self)
        {
            let dirs = self.dirs.read().unwrap();
            for k in dirs.keys() {
                if k == path {
                    continue;
                }
                let key_str = k.to_string_lossy();
                if key_str.starts_with(&prefix) {
                    if let Some(name) = direct_child_name(&key_str, &prefix) {
                        let mut child = path.to_path_buf();
                        child.push(name);
                        entries.push(child);
                    }
                }
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
        self.dirs
            .write()
            .unwrap()
            .retain(|k, _| k == path || !k.to_string_lossy().starts_with(&prefix));
        // Now remove the target dir itself
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

/// Drop guard that auto-resets global fs on drop (panic-safe).
///
/// Also acquires a global test mutex to serialize tests that use
/// `set_global_fs`, preventing race conditions from parallel test execution.
///
/// Use in tests instead of manual `set_global_fs`/`reset_global_fs`:
/// ```ignore
/// let _guard = FsGuard::new();
/// // ... test code ...
/// // guard drops here, resetting global fs even on panic
/// ```
pub struct FsGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl FsGuard {
    pub fn new() -> Self {
        // Reset is not needed here — caller sets up their own fs.
        // The guard ensures cleanup on drop.
        let _lock = GLOBAL_FS_TEST_LOCK.lock().unwrap();
        FsGuard { _lock }
    }
}

impl Drop for FsGuard {
    fn drop(&mut self) {
        reset_global_fs();
    }
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
        assert!(fs.exists(Path::new("/some"))); // parent also created
    }

    #[test]
    fn memfs_read_dir_direct_children_only() {
        let fs = MemFs::new();
        fs.create_dir_all(Path::new("/project")).unwrap();
        fs.write_file("/project/a.txt", "a");
        fs.write_file("/project/b.txt", "b");
        fs.write_file("/other/c.txt", "c");

        let entries = fs.read_dir(Path::new("/project"));
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn memfs_read_dir_no_grandchildren() {
        let fs = MemFs::new();
        fs.create_dir_all(Path::new("/project/sub")).unwrap();
        fs.write_file("/project/a.txt", "a");
        fs.write_file("/project/sub/deep.txt", "d");

        let entries = fs.read_dir(Path::new("/project"));
        // Should return: a.txt and sub/ — NOT sub/deep.txt
        assert_eq!(entries.len(), 2);
        let names: Vec<_> = entries
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"sub"));
    }

    #[test]
    fn memfs_remove_dir_all_cleans_subdirs() {
        let fs = MemFs::new();
        fs.create_dir_all(Path::new("/project/sub")).unwrap();
        fs.write_file("/project/a.txt", "a");
        fs.write_file("/project/sub/b.txt", "b");

        assert!(fs.exists(Path::new("/project/a.txt")));
        assert!(fs.exists(Path::new("/project/sub/b.txt")));

        fs.remove_dir_all(Path::new("/project")).unwrap();

        assert!(!fs.exists(Path::new("/project/a.txt")));
        assert!(!fs.exists(Path::new("/project/sub/b.txt")));
        assert!(!fs.exists(Path::new("/project/sub")));
        assert!(!fs.exists(Path::new("/project")));
    }
}
