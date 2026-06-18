use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub default_author: Option<String>,
    pub default_license: Option<String>,
    pub year_override: Option<String>,
}

impl Config {
    /// Load configuration from XDG config dir / licencify / config.toml
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Save configuration to the same XDG config path
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Get the effective author: config default -> git config user.name -> error
    pub fn effective_author(&self) -> Result<String> {
        // 1. Check config
        if let Some(ref author) = self.default_author {
            if !author.trim().is_empty() {
                return Ok(author.clone());
            }
        }

        // 2. Check git config
        let output = std::process::Command::new("git")
            .args(["config", "user.name"])
            .output()
            .context("Failed to run `git config user.name`. Is git installed?")?;

        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return Ok(name);
            }
        }

        anyhow::bail!(
            "No author specified. Set one via `licencify config --author <name>` \
             or configure git with `git config user.name \"Your Name\"`"
        )
    }

    /// Return the path to the config file: $XDG_CONFIG_HOME/licencify/config.toml
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine XDG config directory")?;
        Ok(config_dir.join("licencify").join("config.toml"))
    }
}
