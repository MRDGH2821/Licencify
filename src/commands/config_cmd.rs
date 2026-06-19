use crate::{
    cli::ConfigAction,
    config::{self, Config},
};
use anyhow::Result;

pub fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Init => cmd_config_init(),
        ConfigAction::Show => cmd_config_show(),
        ConfigAction::Get { key } => cmd_config_get(&key),
        ConfigAction::Set { key, value } => cmd_config_set(&key, &value),
    }
}

pub fn cmd_schema(output: Option<&str>) -> Result<()> {
    let json = Config::schema_json()?;

    match output {
        Some(path) => {
            std::fs::write(path, &json)?;
            println!("✅ Schema written to {}", path);
        }
        None => {
            println!("{}", json);
        }
    }
    Ok(())
}

/// Write schema JSON to the global config dir and (if it exists) alongside
/// the project config.
fn write_schema_file() -> Result<()> {
    let json = Config::schema_json()?;

    // --- global schema ---
    let global_schema = Config::schema_path()?;
    if let Some(parent) = global_schema.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&global_schema, &json)?;
    println!("✅ Schema written: {}", global_schema.display());

    // --- project schema (next to licencify.toml) ---
    let project_cfg = Config::project_path()?;
    if project_cfg.exists() {
        if let Some(parent) = project_cfg.parent() {
            let proj_schema = parent.join("licencify-schema.json");
            std::fs::write(&proj_schema, &json)?;
            println!("✅ Schema written: {}", proj_schema.display());
        }
    }

    Ok(())
}

// ─── init ────────────────────────────────────────────────────────────────────

fn cmd_config_init() -> Result<()> {
    let project_path = Config::project_path()?;
    let global_path = Config::global_path()?;

    // Create global config if it doesn't exist yet
    if !global_path.exists() {
        let global = Config::default();
        global.save()?;
        println!("✅ Created global config: {}", global_path.display());
    }

    // Create project config if it doesn't exist yet
    if project_path.exists() {
        println!("Project config already exists: {}", project_path.display());
        println!("Use `licencify config show` to view current configuration.");
        return Ok(());
    }

    let config = Config::default();
    save_project_config(&config, &project_path)?;

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
    println!("  [subdirs]  (optional)");
    println!("    \"<path>\"       Per-subdirectory overrides (author, license, …)");
    println!();
    println!("Use `licencify config set <key> <value>` to write to global config.");
    println!("Use `licencify config show` to see the effective (merged) configuration.");
    Ok(())
}

// ─── show ────────────────────────────────────────────────────────────────────

fn cmd_config_show() -> Result<()> {
    let global_path = Config::global_path()?;
    let project_path = Config::project_path()?;

    let global_exists = global_path.exists();
    let project_exists = project_path.exists();

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
    let config = Config::load_effective()?;
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

    // [subdirs] — only relevant when a project config is present
    if project_exists {
        println!("[subdirs]");
        match &config.subdirs {
            Some(subdirs) if !subdirs.is_empty() => {
                for (dir, subdir_cfg) in subdirs {
                    println!("  [\"{}\"]", dir);
                    if let Some(author) = &subdir_cfg.author {
                        println!("    author      = {}", author);
                    }
                    if let Some(license) = &subdir_cfg.license {
                        println!("    license     = {}", license);
                    }
                    if let Some(format) = &subdir_cfg.format {
                        println!("    format      = {}", format);
                    }
                    if let Some(year) = &subdir_cfg.year {
                        println!("    year        = {}", year);
                    }
                    if let Some(licence_name) = &subdir_cfg.licence_name {
                        println!("    licence_name = {}", licence_name);
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

// ─── get ─────────────────────────────────────────────────────────────────────

fn cmd_config_get(key: &str) -> Result<()> {
    let config = Config::load()?;

    let value = match key {
        "author" => config.default.author,
        "license" => config.default.license,
        "format" => config.default.format,
        "year" => config.default.year,
        "licence_name" => Some(config.licence_name()),
        "template.paths" => config.template.and_then(|t| t.paths).map(|v| v.join(",")),
        _ => {
            anyhow::bail!(
                "Unknown config key: '{}'\n\
                 Valid keys: author, license, format, year, licence_name, template.paths",
                key
            );
        }
    };

    match value {
        Some(v) => println!("{}", v),
        None => println!("(not set)"),
    }
    Ok(())
}

// ─── set ─────────────────────────────────────────────────────────────────────

fn cmd_config_set(key: &str, value: &str) -> Result<()> {
    let mut config = Config::load()?;

    match key {
        "author" => config.default.author = Some(value.to_string()),
        "license" => config.default.license = Some(value.to_string()),
        "format" => {
            if value != "txt" && value != "html" {
                anyhow::bail!("Invalid format: '{}'. Must be 'txt' or 'html'.", value);
            }
            config.default.format = Some(value.to_string());
        }
        "year" => config.default.year = Some(value.to_string()),
        "licence_name" => {
            let upper = value.to_uppercase();
            if upper != "LICENCE" && upper != "LICENSE" {
                anyhow::bail!(
                    "Invalid licence_name: '{}'. Must be 'LICENCE' or 'LICENSE'.",
                    value
                );
            }
            config.default.licence_name = Some(upper);
        }
        "template.paths" => {
            let new_paths: Vec<String> = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let template = config.template.get_or_insert_with(Default::default);
            match &mut template.paths {
                Some(paths) => {
                    for p in new_paths {
                        if !paths.contains(&p) {
                            paths.push(p);
                        }
                    }
                }
                None => {
                    template.paths = Some(new_paths);
                }
            }
        }
        _ => {
            anyhow::bail!(
                "Unknown config key: '{}'\n\
                 Valid keys: author, license, format, year, licence_name, template.paths",
                key
            );
        }
    }

    config.save()?;

    let path = Config::path()?;
    println!("✅ Set {} = {}", key, value);
    println!("   Saved to {}", path.display());
    Ok(())
}

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Write a Config to a project-level file path (.licencify.toml).
/// This is used only for the project config; global config uses `Config::save()`.
fn save_project_config(config: &Config, path: &std::path::Path) -> Result<()> {
    use anyhow::Context;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Build TOML with schema comment header
    let doc: toml_edit::DocumentMut = toml::to_string_pretty(config)
        .context("Failed to serialize config")?
        .parse()
        .context("Failed to parse serialized config")?;

    let schema_path = Config::schema_path()?;
    let header = format!("#:schema {}\n\n", schema_path.display());
    let mut prefixed = header;
    prefixed.push_str(&doc.to_string());

    std::fs::write(path, prefixed)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;
    Ok(())
}
