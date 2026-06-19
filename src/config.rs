use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::fs::global_fs;

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
    crate::licence_name::LicenceName::detect()
        .as_str()
        .to_string()
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
        let fs = global_fs();
        let mut dir = std::env::current_dir().ok()?;
        loop {
            for name in Self::PROJECT_FILENAMES {
                let candidate = dir.join(name);
                if fs.exists(&candidate) {
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
        let fs = global_fs();
        let cwd = std::env::current_dir().context("Could not determine current directory")?;
        // Check CWD for both filenames
        for name in Self::PROJECT_FILENAMES {
            let candidate = cwd.join(name);
            if fs.exists(&candidate) {
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
        let fs = global_fs();
        let path = Self::global_path().ok()?;
        if !fs.exists(&path) {
            return None;
        }
        let text = fs.read_to_string(&path)?;
        toml::from_str(&text).ok()
    }

    /// Load the project config from the project root.
    /// Returns (merged config, project_root path) so callers don't re-walk.
    fn load_project_with_root() -> Option<(Self, PathBuf)> {
        let fs = global_fs();
        let (project_root, path) = Self::find_project_root()?;
        let text = fs.read_to_string(&path)?;
        let config = toml::from_str(&text).ok()?;
        Some((config, project_root))
    }

    /// Load global config only (no merging).
    pub fn load() -> Result<Self> {
        Self::load_global().context("No global config found. Run `licencify config init` first.")
    }

    /// Load the effective config: global + project + subdir overrides merged.
    pub fn load_effective() -> Result<Self> {
        let global = Self::load_global().unwrap_or_default();
        let (project, project_root) = Self::load_project_with_root()
            .map(|(cfg, root)| (cfg, Some(root)))
            .unwrap_or_default();
        let merged = merge(global, project);

        // Apply subdir overrides based on CWD
        let project_root = match project_root {
            Some(root) => root,
            None => return Ok(merged),
        };

        let subdirs = match &merged.subdirs {
            Some(s) if !s.is_empty() => s.clone(),
            _ => return Ok(merged),
        };

        let cwd = std::env::current_dir().context("Could not determine current directory")?;

        let rel = cwd.strip_prefix(&project_root).unwrap_or(&cwd);
        let rel_str = rel.to_string_lossy();

        // Find longest prefix match (avoid allocation per iteration)
        let best = subdirs
            .iter()
            .filter(|(key, _)| {
                rel_str == key.as_str()
                    || (rel_str.len() > key.len()
                        && rel_str.as_bytes().get(key.len()) == Some(&b'/')
                        && rel_str.starts_with(key.as_str()))
            })
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

    /// Save config to an explicit path with schema header.
    /// Used by both `save()` (global) and project config writes.
    pub fn save_to_path(&self, path: &std::path::Path) -> Result<()> {
        use crate::fs::global_fs;
        let fs = global_fs();
        if let Some(parent) = path.parent() {
            fs.create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let doc: toml_edit::DocumentMut = toml::to_string_pretty(self)
            .context("Failed to serialize config")?
            .parse()
            .context("Failed to parse serialized config")?;

        let schema_path = Self::schema_path()?;
        let header = format!("#:schema {}\n\n", schema_path.display());
        let mut prefixed = header;
        prefixed.push_str(&doc.to_string());

        fs.write(path, &prefixed)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }

    /// Save always writes to global config.
    pub fn save(&self) -> Result<()> {
        let path = Self::global_path()?;
        self.save_to_path(&path)
    }

    /// Get the licence file base name (LICENCE or LICENSE).
    /// Resolves: config → locale detection → default LICENCE.
    pub fn licence_name(&self) -> String {
        crate::licence_name::LicenceName::resolve(self.default.licence_name.as_deref())
            .as_str()
            .to_string()
    }

    /// Search configured template paths for a template file matching the SPDX ID.
    pub fn find_custom_template(&self, spdx_id: &str) -> Option<(String, Option<String>, String)> {
        let fs = global_fs();
        let paths = self.template.as_ref()?.paths.as_ref()?;
        let filename = format!("{}.txt", spdx_id);
        let html_filename = format!("{}.html", spdx_id);

        for dir in paths {
            let dir = std::path::Path::new(dir);

            let text_path = dir.join(&filename);
            let html_path = dir.join(&html_filename);

            if let Some(text) = fs.read_to_string(&text_path) {
                let html = fs.read_to_string(&html_path);
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
