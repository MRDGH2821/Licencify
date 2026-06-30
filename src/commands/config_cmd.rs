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

/// Write schema JSON to the global config dir and (if it exists) alongside
/// the project config.
fn write_schema_file() -> Result<()> {
    let fs = global_fs();
    let json = Config::schema_json()?;

    // --- global schema ---
    let global_schema = Config::schema_path()?;
    if let Some(parent) = global_schema.parent() {
        fs.create_dir_all(parent)?;
    }
    fs.write(&global_schema, &json)?;
    println!("✅ Schema written: {}", global_schema.display());

    // --- project schema (next to licencify.toml) ---
    let project_cfg = Config::project_path()?;
    if fs.exists(&project_cfg) {
        if let Some(parent) = project_cfg.parent() {
            let proj_schema = parent.join("licencify-schema.json");
            fs.write(&proj_schema, &json)?;
            println!("✅ Schema written: {}", proj_schema.display());
        }
    }

    Ok(())
}

// ─── init ────────────────────────────────────────────────────────────────────

fn cmd_config_init() -> Result<()> {
    let fs = global_fs();
    let project_path = Config::project_path()?;
    let global_path = Config::global_path()?;

    // Create global config if it doesn't exist yet
    if !fs.exists(&global_path) {
        let global = Config::default();
        global.save()?;
        println!("✅ Created global config: {}", global_path.display());
    }

    // Create project config if it doesn't exist yet
    if fs.exists(&project_path) {
        println!("Project config already exists: {}", project_path.display());
        println!("Use `licencify config show` to view current configuration.");
        return Ok(());
    }

    let config = Config::default();
    config.save_to_path(&project_path)?;

    println!("✅ Created project config: {}", project_path.display());

    // Generate schema alongside config
    write_schema_file()?;

    let detected = config::detect_licence_name();
    println!();
    println!("Available settings:");
    println!("  [default]");
    println!("    author        Copyright holder name");
    println!("    license       Default SPDX license ID");
    println!("    format        Output format (txt or html)");
    println!("    year          Override copyright year");
    println!(
        "    licence_name  File base name: LICENCE or LICENSE (detected: {})",
        detected
    );
    println!();
    println!("  [template]  (optional)");
    println!("    paths         Custom template search paths (array)");
    println!();
    println!("  [[subdirs]]  (optional)");
    println!("    path           Relative directory path (required)");
    println!("    author         Per-subdirectory author override");
    println!("    license        Per-subdirectory license override");
    println!();
    println!("Use `licencify config show` to see the effective (merged) configuration.");
    Ok(())
}

// ─── show ────────────────────────────────────────────────────────────────────

fn cmd_config_show() -> Result<()> {
    let fs = global_fs();
    let global_path = Config::global_path()?;
    let project_path = Config::project_path()?;

    let global_exists = fs.exists(&global_path);
    let project_exists = fs.exists(&project_path);

    // File locations
    println!(
        "Global config:  {} {}",
        if global_exists { "✓" } else { "✗" },
        global_path.display()
    );
    println!(
        "Project config: {} {}",
        if project_exists { "✓" } else { "✗" },
        project_path.display()
    );
    println!();

    if !global_exists && !project_exists {
        println!("No config files found. Run `licencify config init` to create one.");
        return Ok(());
    }

    // Effective (merged + subdir) values
    let config = Config::load_effective(None)?;
    let detected = config::detect_licence_name();

    println!("[default]");
    println!(
        "  author        = {}",
        config.default.author.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  license       = {}",
        config.default.license.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  format        = {}",
        config.default.format.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  year          = {}",
        config.default.year.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  licence_name  = {} (detected: {})",
        config
            .default
            .licence_name
            .as_deref()
            .unwrap_or("(not set)"),
        detected
    );
    println!();

    // [template]
    println!("[template]");
    match &config.template {
        Some(template) => match &template.paths {
            Some(paths) if !paths.is_empty() => {
                println!("  paths =");
                for (i, p) in paths.iter().enumerate() {
                    println!("    [{}] {}", i, p);
                }
            }
            _ => {
                println!("  paths = (not set)");
            }
        },
        None => {
            println!("  (section not configured)");
        }
    }
    println!();

    // [[subdirs]] — only relevant when a project config is present
    if project_exists {
        println!("[[subdirs]]");
        match &config.subdirs {
            Some(subdirs) if !subdirs.is_empty() => {
                for (i, subdir_cfg) in subdirs.iter().enumerate() {
                    if i > 0 {
                        println!("[[subdirs]]");
                    }
                    println!("path        = \"{}\"", subdir_cfg.path);
                    if let Some(author) = &subdir_cfg.author {
                        println!("author      = \"{}\"", author);
                    }
                    if let Some(license) = &subdir_cfg.license {
                        println!("license     = \"{}\"", license);
                    }
                    if let Some(format) = &subdir_cfg.format {
                        println!("format      = \"{}\"", format);
                    }
                    if let Some(year) = &subdir_cfg.year {
                        println!("year        = \"{}\"", year);
                    }
                    if let Some(licence_name) = &subdir_cfg.licence_name {
                        println!("licence_name = \"{}\"", licence_name);
                    }
                }
            }
            _ => {
                println!("  (no sub-directory overrides)");
            }
        }
    }

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
