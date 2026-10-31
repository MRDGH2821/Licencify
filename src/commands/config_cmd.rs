use crate::{
    cli::ConfigAction,
    config::{CONFIG_KEYS, Config},
    fs::global_fs,
};
use anyhow::{Context, Result};

pub fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Init => cmd_config_init(),
        ConfigAction::Get { key } => cmd_config_get(&key),
        ConfigAction::List => cmd_config_list(),
        ConfigAction::Set { assignment } => cmd_config_set(&assignment),
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

fn cmd_config_get(key: &str) -> Result<()> {
    let config = Config::load_effective(None)?;
    if CONFIG_KEYS.contains(&key) {
        println!(
            "{}",
            config_value(&config, key).unwrap_or("(not set)".into())
        );
    } else {
        eprintln!("Warning: unknown config key `{key}`; ignored");
    }
    Ok(())
}

fn cmd_config_list() -> Result<()> {
    let fs = global_fs();
    for (name, path) in [
        ("Global", Config::global_path()?),
        ("Project", Config::project_path()?),
        ("Local", Config::local_path()?),
    ] {
        println!(
            "{name} config: {} {}",
            if fs.exists(&path) { "✓" } else { "✗" },
            path.display()
        );
    }
    let config = Config::load_effective(None)?;
    println!("[default]");
    print!("{}", toml::to_string(&config.default)?);
    Ok(())
}

fn cmd_config_set(assignment: &str) -> Result<()> {
    let (key, value) = assignment
        .split_once('=')
        .context("Expected config assignment in the form <option>=<value>")?;
    if Config::set_value(key, value)? {
        println!("Updated configuration `{key}`");
    }
    Ok(())
}

fn config_value(config: &Config, key: &str) -> Option<String> {
    let default = &config.default;
    Some(match key {
        "author" => default.author.as_deref()?.to_owned(),
        "company" => default.company.as_deref()?.to_owned(),
        "email" => default.email.as_deref()?.to_owned(),
        "licence" => default.license.as_deref()?.to_owned(),
        "format" => default.format.as_deref()?.to_owned(),
        "year" => default.year.as_deref()?.to_owned(),
        "basename" => default.basename.as_deref()?.to_owned(),
        "update-readme" => default.update_readme?.to_string(),
        "update-manifest" => default.update_manifest?.to_string(),
        "additional-licences" => format!("{:?}", default.additional_licences.as_ref()?),
        _ => return None,
    })
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
    fn cmd_config_list_succeeds_without_config() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        let result = cmd_config(ConfigAction::List);
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
