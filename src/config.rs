use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(
    title = "Licencify Config",
    description = "Configuration file for licencify"
)]
pub struct Config {
    #[schemars(description = "Default values for licence creation")]
    pub default: DefaultConfig,
}

#[derive(Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct DefaultConfig {
    /// Copyright holder name (used as default author for licence files)
    #[schemars(description = "Copyright holder name for licence files")]
    pub author: Option<String>,

    /// Default SPDX license ID (e.g. MIT, Apache-2.0)
    #[schemars(description = "Default SPDX license identifier")]
    pub license: Option<String>,

    /// Output format: txt or html
    #[schemars(description = "Output format for licence files")]
    pub format: Option<String>,

    /// Override copyright year instead of using current year
    #[schemars(description = "Override copyright year (YYYY)")]
    pub year: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default: DefaultConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from config dir / licencify / config.toml
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

    /// Save configuration to the same config path
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        // Build TOML with schema comment header using toml_edit
        let doc: toml_edit::DocumentMut = toml::to_string_pretty(self)
            .context("Failed to serialize config")?
            .parse()
            .context("Failed to parse serialized config")?;

        // Prepend schema comment
        let schema_path = Self::schema_path()?;
        let header = format!("#:schema {}\n\n", schema_path.display());
        let mut prefixed = header;
        prefixed.push_str(&doc.to_string());

        std::fs::write(&path, prefixed)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }

    /// Return the path to the config file
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;
        Ok(config_dir.join("licencify").join("config.toml"))
    }

    /// Return config file path for display purposes
    pub fn path() -> Result<PathBuf> {
        Self::config_path()
    }

    /// Generate JSON Schema for the config struct
    pub fn schema_json() -> Result<String> {
        let schema = schemars::schema_for!(Config);
        serde_json::to_string_pretty(&schema).context("Failed to serialize JSON schema")
    }

    /// Path for the schema file (next to config file)
    pub fn schema_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;
        Ok(config_dir.join("licencify").join("config-schema.json"))
    }
}
