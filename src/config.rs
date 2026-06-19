use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(
    title = "Licencify Config",
    description = "Configuration file for licencify"
)]
pub struct Config {
    #[schemars(description = "Default values for licence creation")]
    pub default: DefaultConfig,

    #[schemars(description = "Template configuration")]
    pub template: Option<TemplateConfig>,

    #[schemars(description = "Sub-directory license overrides (key = relative path)")]
    pub subdirs: Option<HashMap<String, SubdirConfig>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
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

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub struct TemplateConfig {
    /// Custom template search paths (checked before built-in templates)
    #[schemars(description = "Custom template search paths")]
    pub paths: Option<Vec<String>>,
}

/// Sub-directory license override.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
pub struct SubdirConfig {
    pub author: Option<String>,
    pub license: Option<String>,
    pub format: Option<String>,
    pub year: Option<String>,
    pub licence_name: Option<String>,
}

/// Detect whether the system locale uses en-GB or en-US spelling.
/// Returns "LICENCE" for en-GB and "LICENSE" for en-US/other.
pub fn detect_licence_name() -> String {
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
            if lower.starts_with("en") {
                return "LICENSE".to_string();
            }
        }
    }
    "LICENCE".to_string()
}

/// Merge two configs: `overriding` values take priority over `base`.
/// Only `Some` values in `overriding` replace `base`.
fn merge(base: Config, overriding: Config) -> Config {
    Config {
        default: DefaultConfig {
            author: overriding.default.author.or(base.default.author),
            license: overriding.default.license.or(base.default.license),
            format: overriding.default.format.or(base.default.format),
            year: overriding.default.year.or(base.default.year),
            licence_name: overriding
                .default
                .licence_name
                .or(base.default.licence_name),
        },
        template: match (base.template, overriding.template) {
            (Some(base_t), Some(over_t)) => {
                let paths = over_t.paths.or(base_t.paths);
                Some(TemplateConfig { paths })
            }
            (None, Some(t)) => Some(t),
            (Some(t), None) => Some(t),
            (None, None) => None,
        },
        // Subdirs: overriding replaces entirely if present
        subdirs: overriding.subdirs.or(base.subdirs),
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default: DefaultConfig::default(),
            template: None,
            subdirs: None,
        }
    }
}

impl Config {
    /// Return the global config file path.
    pub fn global_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;
        Ok(config_dir.join("licencify").join("config.toml"))
    }

    const PROJECT_FILENAMES: [&'static str; 2] = [".licencify.toml", "licencify.toml"];

    /// Walk up from CWD to find the directory containing a project config.
    /// Checks `.licencify.toml` first, then `licencify.toml`.
    /// Returns (project_root, project_config_path) if found.
    fn find_project_root() -> Option<(PathBuf, PathBuf)> {
        let mut dir = std::env::current_dir().ok()?;
        loop {
            for name in Self::PROJECT_FILENAMES {
                let candidate = dir.join(name);
                if candidate.exists() {
                    return Some((dir, candidate));
                }
            }
            if !dir.pop() {
                break;
            }
        }
        None
    }

    /// Return the project-level config file path.
    /// Checks CWD for `.licencify.toml` first, then `licencify.toml`, then walks up.
    pub fn project_path() -> Result<PathBuf> {
        let cwd = std::env::current_dir().context("Could not determine current directory")?;
        // Check CWD for both filenames
        for name in Self::PROJECT_FILENAMES {
            let candidate = cwd.join(name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        // Walk up to find it
        if let Some((_, path)) = Self::find_project_root() {
            return Ok(path);
        }
        // Return default name even if it doesn't exist (for init, etc.)
        Ok(cwd.join(".licencify.toml"))
    }

    /// Legacy path() — returns global path for backwards compat.
    pub fn path() -> Result<PathBuf> {
        Self::global_path()
    }

    /// Load the global config from config dir.
    fn load_global() -> Option<Self> {
        let path = Self::global_path().ok()?;
        if !path.exists() {
            return None;
        }
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }

    /// Load the project-level config by walking up from CWD.
    fn load_project() -> Option<Self> {
        let (_, path) = Self::find_project_root()?;
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }

    /// Load merged config: global + project-level (project overrides global).
    pub fn load() -> Result<Self> {
        let global = Self::load_global().unwrap_or_default();
        let project = Self::load_project().unwrap_or_default();
        Ok(merge(global, project))
    }

    /// Load the effective config, including subdir resolution.
    /// 1. Merge global + project
    /// 2. Find project root (dir containing licencify.toml)
    /// 3. Compute relative CWD from project root
    /// 4. Find longest-prefix match in [subdirs]
    /// 5. Merge matched subdir config on top
    pub fn load_effective() -> Result<Self> {
        let merged = Self::load()?;

        // Find project root and check for subdirs
        let (project_root, _project_path) = match Self::find_project_root() {
            Some((root, path)) => (root, path),
            None => return Ok(merged),
        };

        let subdirs = match &merged.subdirs {
            Some(s) if !s.is_empty() => s.clone(),
            _ => return Ok(merged),
        };

        let cwd = std::env::current_dir().context("Could not determine current directory")?;

        let rel = cwd.strip_prefix(&project_root).unwrap_or(&cwd);
        let rel_str = rel.to_string_lossy();

        // Find longest prefix match
        let best = subdirs
            .iter()
            .filter(|(key, _)| rel_str == key.as_str() || rel_str.starts_with(&format!("{}/", key)))
            .max_by_key(|(key, _)| key.len());

        if let Some((matched_path, subdir_cfg)) = best {
            // Only apply if at least one field is set
            if subdir_cfg.author.is_some()
                || subdir_cfg.license.is_some()
                || subdir_cfg.format.is_some()
                || subdir_cfg.year.is_some()
                || subdir_cfg.licence_name.is_some()
            {
                let default_override = DefaultConfig {
                    author: subdir_cfg.author.clone(),
                    license: subdir_cfg.license.clone(),
                    format: subdir_cfg.format.clone(),
                    year: subdir_cfg.year.clone(),
                    licence_name: subdir_cfg.licence_name.clone(),
                };
                let override_config = Config {
                    default: default_override,
                    template: None,
                    subdirs: None,
                };
                let effective = merge(merged, override_config);
                eprintln!("[licencify] sub-dir override active: \"{}\"", matched_path);
                return Ok(effective);
            }
        }

        Ok(merged)
    }

    /// Save always writes to global config.
    pub fn save(&self) -> Result<()> {
        let path = Self::global_path()?;
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

    /// Generate JSON Schema for the config struct
    pub fn schema_json() -> Result<String> {
        let schema = schemars::schema_for!(Config);
        serde_json::to_string_pretty(&schema).context("Failed to serialize JSON schema")
    }

    /// Path for the schema file (next to global config file)
    pub fn schema_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;
        Ok(config_dir.join("licencify").join("licencify-schema.json"))
    }
}
