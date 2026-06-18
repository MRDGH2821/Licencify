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

    #[schemars(description = "Template configuration")]
    pub template: Option<TemplateConfig>,
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

    /// Licence file base name: "LICENCE" (en-GB) or "LICENSE" (en-US)
    #[schemars(description = "Licence file base name (LICENCE or LICENSE)")]
    pub licence_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct TemplateConfig {
    /// Custom template search paths (checked before built-in templates)
    #[schemars(description = "Custom template search paths")]
    pub paths: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default: DefaultConfig::default(),
            template: None,
        }
    }
}

/// Detect whether the system locale uses en-GB or en-US spelling.
/// Returns "LICENCE" for en-GB and "LICENSE" for en-US/other.
pub fn detect_licence_name() -> String {
    // Check common locale environment variables
    for var in &["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(val) = std::env::var(var) {
            let lower = val.to_lowercase();
            if lower.contains("en_gb")
                || lower.contains("en-gb")
                || lower.contains("en.au")
                || lower.contains("en_nz")
                || lower.contains("en-in")
            {
                return "LICENCE".to_string();
            }
            // en_US and other English variants default to LICENSE
            if lower.starts_with("en") {
                return "LICENSE".to_string();
            }
        }
    }
    // Default to en-GB (user preference)
    "LICENCE".to_string()
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

        // Build TOML with schema comment header
        let doc: toml_edit::DocumentMut = toml::to_string_pretty(self)
            .context("Failed to serialize config")?
            .parse()
            .context("Failed to parse serialized config")?;

        let schema_path = Self::schema_path()?;
        let header = format!("#:schema {}\n\n", schema_path.display());
        let mut prefixed = header;
        prefixed.push_str(&doc.to_string());

        std::fs::write(&path, prefixed)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }

    /// Get the licence file base name (LICENCE or LICENSE).
    /// Resolves: config → locale detection → default LICENCE.
    pub fn licence_name(&self) -> String {
        self.default
            .licence_name
            .clone()
            .unwrap_or_else(detect_licence_name)
    }

    /// Search configured template paths for a template file matching the SPDX ID.
    pub fn find_custom_template(&self, spdx_id: &str) -> Option<(String, Option<String>, String)> {
        let paths = self.template.as_ref()?.paths.as_ref()?;
        let filename = format!("{}.txt", spdx_id);
        let html_filename = format!("{}.html", spdx_id);

        for dir in paths {
            let dir = std::path::Path::new(dir);
            if !dir.is_dir() {
                continue;
            }

            let text_path = dir.join(&filename);
            let html_path = dir.join(&html_filename);

            if text_path.is_file() {
                let text = std::fs::read_to_string(&text_path).ok()?;
                let html = std::fs::read_to_string(&html_path).ok();
                let source = format!("custom ({})", dir.display());
                return Some((text, html, source));
            }
        }

        None
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
