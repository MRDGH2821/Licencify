use crate::{config::Config, fs::global_fs};
use anyhow::Result;

pub fn cmd_init() -> Result<()> {
    let fs = global_fs();
    let path = Config::project_path()?;
    if fs.exists(&path) {
        println!("Project config already exists: {}", path.display());
        return Ok(());
    }
    Config::default().save_to_path(&path)?;
    println!("Created project config: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{FsGuard, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_init_creates_project_config_with_schema_url() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\n",
        )
        .unwrap();

        let result = cmd_init();
        assert!(result.is_ok());

        let path = Config::project_path().unwrap();
        assert!(fs.exists(&path));
        let content = fs.read_to_string(&path).unwrap();
        assert!(content.starts_with(&format!("#:schema {}\n\n", Config::SCHEMA_URL)));
    }

    #[test]
    fn cmd_init_does_not_overwrite_existing() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\n",
        )
        .unwrap();

        let path = Config::project_path().unwrap();
        if let Some(parent) = path.parent() {
            fs.create_dir_all(parent).unwrap();
        }
        let original = "# custom config\n[default]\nauthor = \"Tester\"\n";
        fs.write(&path, original).unwrap();

        let result = cmd_init();
        assert!(result.is_ok());

        let content = fs.read_to_string(&path).unwrap();
        assert_eq!(content, original);
    }
}
