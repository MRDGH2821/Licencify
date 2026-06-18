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

fn write_schema_file() -> Result<()> {
    let schema_path = Config::schema_path()?;
    let json = Config::schema_json()?;

    if let Some(parent) = schema_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&schema_path, &json)?;
    println!("✅ Schema written: {}", schema_path.display());
    Ok(())
}

fn cmd_config_init() -> Result<()> {
    let path = Config::path()?;

    if path.exists() {
        println!("Config file already exists: {}", path.display());
        println!("Use `licencify config show` to view current configuration.");
        return Ok(());
    }

    let config = Config::default();
    config.save()?;

    println!("✅ Created config file: {}", path.display());

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
    println!("Use `licencify config set <key> <value>` to configure.");
    Ok(())
}

fn cmd_config_show() -> Result<()> {
    let config = Config::load()?;
    let path = Config::path()?;

    println!("Config file: {}", path.display());
    println!();

    if !path.exists() {
        println!("No config file found. Run `licencify config init` to create one.");
        return Ok(());
    }

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
    Ok(())
}

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
