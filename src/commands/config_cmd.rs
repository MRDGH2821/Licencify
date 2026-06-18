use crate::{cli::ConfigAction, config::Config};
use anyhow::Result;

pub fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Init => cmd_config_init(),
        ConfigAction::Show => cmd_config_show(),
        ConfigAction::Get { key } => cmd_config_get(&key),
        ConfigAction::Set { key, value } => cmd_config_set(&key, &value),
    }
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
    println!();
    println!("Available settings:");
    println!("  default_author   Copyright holder name");
    println!("  default_license  Default SPDX license ID");
    println!("  default_format   Output format (txt or html)");
    println!("  year_override    Override copyright year");
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

    println!(
        "default_author  = {}",
        config.default_author.as_deref().unwrap_or("(not set)")
    );
    println!(
        "default_license = {}",
        config.default_license.as_deref().unwrap_or("(not set)")
    );
    println!(
        "default_format  = {}",
        config.default_format.as_deref().unwrap_or("(not set)")
    );
    println!(
        "year_override   = {}",
        config.year_override.as_deref().unwrap_or("(not set)")
    );
    Ok(())
}

fn cmd_config_get(key: &str) -> Result<()> {
    let config = Config::load()?;

    let value = match key {
        "default_author" => config.default_author,
        "default_license" => config.default_license,
        "default_format" => config.default_format,
        "year_override" => config.year_override,
        _ => {
            anyhow::bail!(
                "Unknown config key: '{}'\n\
                 Valid keys: default_author, default_license, default_format, year_override",
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
        "default_author" => config.default_author = Some(value.to_string()),
        "default_license" => config.default_license = Some(value.to_string()),
        "default_format" => {
            if value != "txt" && value != "html" {
                anyhow::bail!("Invalid format: '{}'. Must be 'txt' or 'html'.", value);
            }
            config.default_format = Some(value.to_string());
        }
        "year_override" => config.year_override = Some(value.to_string()),
        _ => {
            anyhow::bail!(
                "Unknown config key: '{}'\n\
                 Valid keys: default_author, default_license, default_format, year_override",
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
