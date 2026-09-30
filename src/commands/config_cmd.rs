use crate::{
    cli::ConfigAction,
    config::{self, Config},
    fs::global_fs,
};
use anyhow::Result;

pub fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Init => cmd_config_init(),
        ConfigAction::Show => cmd_config_show(),
    }
}

pub fn cmd_schema(output: &str) -> Result<()> {
    let json = Config::schema_json()?;
    let fs = global_fs();
    fs.write(std::path::Path::new(output), &json)?;
    println!("✅ Schema written to {}", output);
    Ok(())
}

fn cmd_config_init() -> Result<()> {
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

fn cmd_config_show() -> Result<()> {
    let fs = global_fs();
    let global_path = Config::global_path()?;
    let project_path = Config::project_path()?;
    let local_path = Config::local_path()?;
    println!(
        "Global config: {} {}",
        if fs.exists(&global_path) {
            "✓"
        } else {
            "✗"
        },
        global_path.display()
    );
    println!(
        "Project config: {} {}",
        if fs.exists(&project_path) {
            "✓"
        } else {
            "✗"
        },
        project_path.display()
    );
    println!(
        "Local config: {} {}",
        if fs.exists(&local_path) { "✓" } else { "✗" },
        local_path.display()
    );
    let config = Config::load_effective(None)?;
    println!("[default]");
    println!(
        "  author = {}",
        config.default.author.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  company = {}",
        config.default.company.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  email = {}",
        config.default.email.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  licence = {}",
        config.default.license.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  format = {}",
        config.default.format.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  year = {}",
        config.default.year.as_deref().unwrap_or("(not set)")
    );
    println!("  update_readme = {:?}", config.default.update_readme);
    println!(
        "  additional-licences = {:?}",
        config.default.additional_licences
    );
    println!(
        "  licence_file_name = {}",
        config
            .default
            .licence_name
            .as_deref()
            .unwrap_or("(not set)")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ConfigAction;
    use crate::fs::{FsGuard, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_config_init_creates_project_config() {
        let _guard = FsGuard::new();
        // Need Cargo.toml so config init can detect the project root
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("Cargo.toml"),
            "[package]\nname = \"test\"\n",
        )
        .unwrap();
        let result = cmd_config(ConfigAction::Init);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_config_show_succeeds_without_config() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        let result = cmd_config(ConfigAction::Show);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_schema_writes_file() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        let result = cmd_schema("test-schema.json");
        assert!(result.is_ok());
        assert!(fs.exists(std::path::Path::new("test-schema.json")));
    }
}
