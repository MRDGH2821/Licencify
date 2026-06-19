use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

    #[schemars(description = "Sub-directory licence overrides")]
    pub subdirs: Option<Vec<SubdirConfig>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub struct DefaultConfig {
    /// Copyright holder name (used as default author for licence files)
    #[schemars(description = "Copyright holder name for licence files")]
    pub author: Option<String>,

    /// Company name (defaults to author if not set)
    #[schemars(description = "Company name for proprietary notices")]
    pub company: Option<String>,

    /// Contact email address
    #[schemars(description = "Contact email for proprietary notices")]
    pub email: Option<String>,

    /// Default SPDX license ID (e.g. MIT, Apache-2.0, proprietary)
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

/// Sub-directory licence override.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
pub struct SubdirConfig {
    /// Relative path to the sub-directory (relative to config file location)
    #[schemars(description = "Relative path to the sub-directory")]
    pub path: String,

    /// Copyright holder override for this sub-directory
    #[schemars(description = "Copyright holder override")]
    pub author: Option<String>,

    /// Company name override for this sub-directory
    #[schemars(description = "Company name override")]
    pub company: Option<String>,

    /// Email address override for this sub-directory
    #[schemars(description = "Email address override")]
    pub email: Option<String>,

    /// SPDX license ID override for this sub-directory
    #[schemars(description = "SPDX license identifier override")]
    pub license: Option<String>,

    /// Output format override for this sub-directory
    #[schemars(description = "Output format override")]
    pub format: Option<String>,

    /// Copyright year override for this sub-directory
    #[schemars(description = "Copyright year override")]
    pub year: Option<String>,

    /// Licence file base name override for this sub-directory
    #[schemars(description = "Licence file base name override")]
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
            company: overriding.default.company.or(base.default.company),
            email: overriding.default.email.or(base.default.email),
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

    /// Load the global config from config dir.
    fn load_global() -> Option<Self> {
        let path = Self::global_path().ok()?;
        let fs = global_fs();
        let content = fs.read_to_string(&path)?;
        toml::from_str(&content).ok()
    }

    /// Load effective config with explicit project root.
    /// Used by callers who already know the project root (avoids double walk).
    pub fn load_project_with_root(root: &std::path::Path) -> Result<(Self, PathBuf)> {
        let fs = global_fs();

        // Try both project config filenames
        for name in Self::PROJECT_FILENAMES {
            let candidate = root.join(name);
            if fs.exists(&candidate) {
                let content = fs
                    .read_to_string(&candidate)
                    .with_context(|| format!("Failed to read {}", candidate.display()))?;
                let project: Self =
                    toml::from_str(&content).with_context(|| "Failed to parse project config")?;

                let global = Self::load_global();
                let merged = match global {
                    Some(g) => merge(g, project),
                    None => project,
                };
                return Ok((merged, root.to_path_buf()));
            }
        }

        anyhow::bail!("No project config found in {}", root.display())
    }

    /// Load effective config with subdirectory overrides applied.
    /// Resolves the project root, then applies the best-matching subdir override.
    pub fn load_effective(subdir: Option<&str>) -> Result<Self> {
        let (merged, _root) = Self::load_project_with_root(
            &Self::find_project_root()
                .map(|(r, _)| r)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
        )?;

        // Apply sub-directory overrides if a subdir is specified
        if let Some(subdir_path) = subdir {
            let subdirs = match &merged.subdirs {
                Some(s) if !s.is_empty() => s,
                _ => return Ok(merged),
            };

            // Find the best matching subdir override (longest prefix match)
            let best = subdirs
                .iter()
                .filter(|s| {
                    let key = s.path.trim_end_matches('/');
                    let rel_str = subdir_path.trim_end_matches('/');
                    rel_str == key
                        || (rel_str.len() > key.len()
                            && rel_str.as_bytes().get(key.len()) == Some(&b'/')
                            && rel_str.starts_with(key))
                })
                .max_by_key(|s| s.path.len());

            if let Some(subdir_cfg) = best {
                // Only apply if at least one field is set
                if subdir_cfg.author.is_some()
                    || subdir_cfg.company.is_some()
                    || subdir_cfg.email.is_some()
                    || subdir_cfg.license.is_some()
                    || subdir_cfg.format.is_some()
                    || subdir_cfg.year.is_some()
                    || subdir_cfg.licence_name.is_some()
                {
                    let path = subdir_cfg.path.clone();
                    let default_override = DefaultConfig {
                        author: subdir_cfg.author.clone(),
                        company: subdir_cfg.company.clone(),
                        email: subdir_cfg.email.clone(),
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
                    eprintln!("[licencify] sub-dir override active: \"{}\"", path);
                    return Ok(effective);
                }
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

    /// Update the project config's `[default]` section with the given values.
    /// Only writes if a project config file exists; preserves `template` and `subdirs`.
    pub fn update_project_defaults(license: &str, author: &str, format: &str) -> Result<bool> {
        let (_, path) = match Self::find_project_root() {
            Some(p) => p,
            None => return Ok(false),
        };
        let fs = global_fs();
        let content = fs
            .read_to_string(&path)
            .ok_or_else(|| anyhow::anyhow!("Failed to read project config: {}", path.display()))?;
        let mut project: Config = toml::from_str(&content)?;
        project.default.license = Some(license.to_string());
        project.default.author = Some(author.to_string());
        project.default.format = Some(format.to_string());
        project.save_to_path(&path)?;
        Ok(true)
    }

    /// Save always writes to global config.
    pub fn save(&self) -> Result<()> {
        let path = Self::global_path()?;
        self.save_to_path(&path)
    }

    /// Get the licence name setting from config (if configured).
    /// Used by callers who need `Option<&str>` for LicenceName::resolve().
    pub fn licence_name_setting(&self) -> Option<&str> {
        self.default.licence_name.as_deref()
    }

    /// Search configured template paths for a template file matching the SPDX ID.
    pub fn find_custom_template(&self, spdx_id: &str, format: &str) -> Option<(String, String)> {
        let fs = global_fs();
        let paths = self.template.as_ref()?.paths.as_ref()?;
        let filename = format!("{}.{}", spdx_id, format);

        for dir in paths {
            let dir = std::path::Path::new(dir);
            let text_path = dir.join(&filename);

            if let Some(text) = fs.read_to_string(&text_path) {
                let source = format!("custom ({})", dir.display());
                return Some((text, source));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_overriding_values() {
        let base = Config {
            default: DefaultConfig {
                author: Some("Base Author".into()),
                license: Some("MIT".into()),
                ..Default::default()
            },
            template: None,
            subdirs: None,
        };
        let overriding = Config {
            default: DefaultConfig {
                author: Some("Override Author".into()),
                ..Default::default()
            },
            template: None,
            subdirs: None,
        };
        let merged = merge(base, overriding);
        assert_eq!(merged.default.author.as_deref(), Some("Override Author"));
        assert_eq!(merged.default.license.as_deref(), Some("MIT")); // base preserved
    }

    #[test]
    fn merge_uses_base_when_overriding_is_none() {
        let base = Config {
            default: DefaultConfig {
                author: Some("Base Author".into()),
                license: Some("MIT".into()),
                ..Default::default()
            },
            template: None,
            subdirs: None,
        };
        let overriding = Config::default();
        let merged = merge(base, overriding);
        assert_eq!(merged.default.author.as_deref(), Some("Base Author"));
        assert_eq!(merged.default.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn merge_template_configs() {
        let base = Config {
            default: DefaultConfig::default(),
            template: Some(TemplateConfig {
                paths: Some(vec!["/base/path".into()]),
            }),
            subdirs: None,
        };
        let overriding = Config {
            default: DefaultConfig::default(),
            template: Some(TemplateConfig {
                paths: Some(vec!["/override/path".into()]),
            }),
            subdirs: None,
        };
        let merged = merge(base, overriding);
        let paths = merged.template.unwrap().paths.unwrap();
        assert_eq!(paths, vec!["/override/path"]);
    }

    #[test]
    fn merge_subdirs_replaces_entirely() {
        let base = Config {
            default: DefaultConfig::default(),
            template: None,
            subdirs: Some(vec![SubdirConfig {
                path: "old".into(),
                author: Some("Old".into()),
                ..Default::default()
            }]),
        };
        let overriding = Config {
            default: DefaultConfig::default(),
            template: None,
            subdirs: Some(vec![SubdirConfig {
                path: "new".into(),
                author: Some("New".into()),
                ..Default::default()
            }]),
        };
        let merged = merge(base, overriding);
        let subdirs = merged.subdirs.unwrap();
        assert_eq!(subdirs.len(), 1);
        assert_eq!(subdirs[0].path, "new");
    }

    #[test]
    fn schema_json_is_valid_json() {
        let schema = Config::schema_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&schema).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("properties").is_some());
    }

    #[test]
    fn config_default_is_empty() {
        let config = Config::default();
        assert!(config.default.author.is_none());
        assert!(config.default.license.is_none());
        assert!(config.template.is_none());
        assert!(config.subdirs.is_none());
    }

    #[test]
    fn licence_name_setting_returns_some_when_configured() {
        let config = Config {
            default: DefaultConfig {
                licence_name: Some("LICENSE".into()),
                ..Default::default()
            },
            template: None,
            subdirs: None,
        };
        assert_eq!(config.licence_name_setting(), Some("LICENSE"));
    }

    #[test]
    fn licence_name_setting_returns_none_when_not_configured() {
        let config = Config::default();
        assert!(config.licence_name_setting().is_none());
    }

    #[test]
    fn subdir_config_has_path_field() {
        let subdir = SubdirConfig {
            path: "src/lib".into(),
            author: Some("Alice".into()),
            license: Some("BSD-2-Clause".into()),
            ..Default::default()
        };
        assert_eq!(subdir.path, "src/lib");
        assert_eq!(subdir.author.as_deref(), Some("Alice"));
        assert_eq!(subdir.license.as_deref(), Some("BSD-2-Clause"));
    }

    #[test]
    fn subdirs_serde_roundtrip() {
        let config = Config {
            default: DefaultConfig {
                author: Some("Test".into()),
                ..Default::default()
            },
            template: None,
            subdirs: Some(vec![
                SubdirConfig {
                    path: "src/lib".into(),
                    author: Some("Alice".into()),
                    license: Some("BSD-2-Clause".into()),
                    ..Default::default()
                },
                SubdirConfig {
                    path: "tests".into(),
                    license: Some("Apache-2.0".into()),
                    ..Default::default()
                },
            ]),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        let subdirs = parsed.subdirs.unwrap();
        assert_eq!(subdirs.len(), 2);
        assert_eq!(subdirs[0].path, "src/lib");
        assert_eq!(subdirs[0].author.as_deref(), Some("Alice"));
        assert_eq!(subdirs[1].path, "tests");
        assert_eq!(subdirs[1].license.as_deref(), Some("Apache-2.0"));
    }
}
