use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::fs::global_fs;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(
    title = "Licencify Config",
    description = "Configuration file for licencify"
)]
#[serde(default)]
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
    #[serde(rename = "licence", alias = "license")]
    pub license: Option<String>,

    /// Output format: txt or html
    #[schemars(description = "Output format for licence files")]
    pub format: Option<String>,

    /// Override copyright year instead of using current year
    #[schemars(description = "Override copyright year (YYYY)")]
    pub year: Option<String>,

    /// Licence file base name: "LICENCE" (en-GB) or "LICENSE" (en-US)
    #[schemars(description = "Licence file base name (LICENCE or LICENSE)")]
    #[serde(rename = "licence_file_name", alias = "licence_name")]
    pub licence_name: Option<String>,

    /// Automatically update README with license badge on add/update
    #[schemars(description = "Automatically update README with license badge")]
    pub update_readme: Option<bool>,
    /// Additional SPDX licence identifiers.
    #[schemars(description = "Additional SPDX licence identifiers")]
    #[serde(rename = "additional-licences", alias = "additional_licences")]
    pub additional_licences: Option<Vec<String>>,
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

    /// SPDX license ID override
    #[schemars(description = "SPDX license identifier override")]
    #[serde(rename = "licence", alias = "license")]
    pub license: Option<String>,

    /// Output format override
    #[schemars(description = "Output format override")]
    pub format: Option<String>,

    /// Copyright year override
    #[schemars(description = "Copyright year override")]
    pub year: Option<String>,

    /// Licence file base name override
    #[schemars(description = "Licence file base name override")]
    #[serde(rename = "licence_file_name", alias = "licence_name")]
    pub licence_name: Option<String>,

    /// Additional SPDX licence identifiers override
    #[schemars(description = "Additional SPDX licence identifiers override")]
    #[serde(rename = "additional-licences", alias = "additional_licences")]
    pub additional_licences: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GlobalConfig {
    #[serde(default)]
    default: DefaultConfig,
    template: Option<TemplateConfig>,
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
            update_readme: overriding
                .default
                .update_readme
                .or(base.default.update_readme),
            additional_licences: overriding
                .default
                .additional_licences
                .or(base.default.additional_licences),
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
    pub fn global_path() -> Result<PathBuf> {
        let dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(dirs::config_dir)
            .context("Could not determine config directory")?;
        Ok(dir.join("licencify/config.toml"))
    }

    pub fn project_path() -> Result<PathBuf> {
        Ok(Self::project_config_dir()?.join("config.toml"))
    }

    pub fn local_path() -> Result<PathBuf> {
        Ok(Self::project_config_dir()?.join("config.local.toml"))
    }

    fn project_config_dir() -> Result<PathBuf> {
        let root = Self::project_root()?;
        Ok(std::env::var_os("PRJ_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(".config"))
            .join("licencify"))
    }

    pub(crate) fn project_root() -> Result<PathBuf> {
        let cwd = std::env::current_dir().context("Could not determine current directory")?;
        let explicit_root = std::env::var_os("PRJ_ROOT")
            .map(PathBuf::from)
            .map(|root| {
                anyhow::ensure!(root.is_absolute(), "PRJ_ROOT must be absolute");
                root.canonicalize().context("Could not resolve PRJ_ROOT")
            })
            .transpose()?;
        let (git_root, config_root) = if explicit_root.is_some() {
            (None, None)
        } else if let Some(root) = Self::git_worktree_root(&cwd) {
            (Some(root), None)
        } else {
            let mut found = None;
            for ancestor in cwd.ancestors() {
                if global_fs().exists(&ancestor.join(".config/licencify/config.toml")) {
                    found = Some(ancestor.to_path_buf());
                }
            }
            (None, found)
        };
        let cwd = if explicit_root.is_some() {
            cwd.canonicalize()
                .context("Could not resolve current directory")?
        } else {
            cwd
        };
        Self::select_project_root(&cwd, explicit_root, git_root, config_root)
    }

    fn git_worktree_root(cwd: &Path) -> Option<PathBuf> {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(cwd)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let root = String::from_utf8(output.stdout).ok()?;
        let root = PathBuf::from(root.trim_end());
        root.is_absolute().then_some(root)
    }

    fn select_project_root(
        cwd: &Path,
        explicit_root: Option<PathBuf>,
        git_root: Option<PathBuf>,
        config_root: Option<PathBuf>,
    ) -> Result<PathBuf> {
        if let Some(root) = explicit_root {
            anyhow::ensure!(root.is_absolute(), "PRJ_ROOT must be absolute");
            anyhow::ensure!(
                cwd.starts_with(&root),
                "Current directory is outside PRJ_ROOT"
            );
            return Ok(root);
        }
        Ok(git_root
            .or(config_root)
            .unwrap_or_else(|| cwd.to_path_buf()))
    }

    fn warn_unknown(value: &toml::Value, path: &std::path::Path, global: bool) {
        let allowed_top = if global {
            &["default", "template"][..]
        } else {
            &["default", "template", "subdirs"][..]
        };
        if let Some(table) = value.as_table() {
            for key in table
                .keys()
                .filter(|key| !allowed_top.contains(&key.as_str()))
            {
                eprintln!("Warning: {}: unknown config key `{key}`", path.display());
            }
            if let Some(default) = table.get("default").and_then(toml::Value::as_table) {
                for key in default.keys().filter(|key| {
                    ![
                        "author",
                        "company",
                        "email",
                        "licence",
                        "license",
                        "year",
                        "format",
                        "update_readme",
                        "licence_file_name",
                        "licence_name",
                        "additional-licences",
                        "additional_licences",
                    ]
                    .contains(&key.as_str())
                }) {
                    eprintln!(
                        "Warning: {}: unknown config key `default.{key}`",
                        path.display()
                    );
                }
            }
            if let Some(template) = table.get("template").and_then(toml::Value::as_table) {
                for key in template.keys().filter(|key| *key != "paths") {
                    eprintln!(
                        "Warning: {}: unknown config key `template.{key}`",
                        path.display()
                    );
                }
            }
            if let Some(subdirs) = table.get("subdirs").and_then(toml::Value::as_array) {
                for (index, entry) in subdirs.iter().enumerate() {
                    if let Some(entry) = entry.as_table() {
                        for key in entry.keys().filter(|key| {
                            ![
                                "path",
                                "author",
                                "company",
                                "email",
                                "licence",
                                "license",
                                "year",
                                "format",
                                "licence_file_name",
                                "licence_name",
                                "additional-licences",
                                "additional_licences",
                            ]
                            .contains(&key.as_str())
                        }) {
                            eprintln!(
                                "Warning: {}: unknown config key `subdirs[{index}].{key}`",
                                path.display()
                            );
                        }
                    }
                }
            }
        }
    }

    fn read_config(path: &std::path::Path, global: bool) -> Result<Option<Self>> {
        let Some(text) = global_fs().read_to_string(path) else {
            return Ok(None);
        };
        let value: toml::Value = toml::from_str(&text)
            .with_context(|| format!("Failed to parse config {}", path.display()))?;
        if global && value.get("subdirs").is_some() {
            anyhow::bail!(
                "Global config {} cannot contain [[subdirs]]",
                path.display()
            );
        }
        Self::warn_unknown(&value, path, global);
        let config = if global {
            let parsed: GlobalConfig = value
                .try_into()
                .with_context(|| format!("Failed to parse config {}", path.display()))?;
            Self {
                default: parsed.default,
                template: parsed.template,
                subdirs: None,
            }
        } else {
            value
                .try_into()
                .with_context(|| format!("Failed to parse config {}", path.display()))?
        };
        Self::validate(&config, path)?;
        Ok(Some(config))
    }

    fn validate(config: &Self, path: &Path) -> Result<()> {
        let needs_spdx_index = config
            .default
            .license
            .as_deref()
            .is_some_and(|value| !value.eq_ignore_ascii_case("proprietary"))
            || config
                .default
                .additional_licences
                .as_ref()
                .is_some_and(|values| !values.is_empty())
            || config.subdirs.as_ref().is_some_and(|entries| {
                entries.iter().any(|entry| {
                    entry
                        .license
                        .as_deref()
                        .is_some_and(|value| !value.eq_ignore_ascii_case("proprietary"))
                        || entry
                            .additional_licences
                            .as_ref()
                            .is_some_and(|values| !values.is_empty())
                })
            });
        let spdx_index = needs_spdx_index
            .then(crate::spdx::SpdxIndex::load)
            .transpose()?;
        Self::validate_values(
            config.default.license.as_deref(),
            config.default.format.as_deref(),
            config.default.year.as_deref(),
            config.default.licence_name.as_deref(),
            config.default.additional_licences.as_deref(),
            "[default]",
            path,
            spdx_index.as_ref(),
        )?;
        if let Some(entries) = &config.subdirs {
            for (index, entry) in entries.iter().enumerate() {
                Self::validate_values(
                    entry.license.as_deref(),
                    entry.format.as_deref(),
                    entry.year.as_deref(),
                    entry.licence_name.as_deref(),
                    entry.additional_licences.as_deref(),
                    &format!("subdirs[{index}]"),
                    path,
                    spdx_index.as_ref(),
                )?;
            }
        }
        Ok(())
    }

    fn validate_values(
        license: Option<&str>,
        format: Option<&str>,
        year: Option<&str>,
        licence_name: Option<&str>,
        additional_licences: Option<&[String]>,
        key: &str,
        path: &Path,
        spdx_index: Option<&crate::spdx::SpdxIndex>,
    ) -> Result<()> {
        if let Some(value) = license {
            anyhow::ensure!(
                value.eq_ignore_ascii_case("proprietary")
                    || spdx_index.is_some_and(|index| index.find(value).is_some()),
                "Invalid config value: {} {key}.licence = {:?}",
                path.display(),
                value
            );
        }
        if let Some(values) = additional_licences {
            for value in values {
                anyhow::ensure!(
                    spdx_index.is_some_and(|index| index.find(value).is_some()),
                    "Invalid config value: {} {key}.additional-licences = {:?}",
                    path.display(),
                    value
                );
            }
        }
        if let Some(value) = format {
            anyhow::ensure!(
                ["txt", "html", "md"].contains(&value),
                "Invalid config value: {} {key}.format = {:?}",
                path.display(),
                value
            );
        }
        if let Some(value) = year {
            anyhow::ensure!(
                value.len() == 4 && value.chars().all(|c| c.is_ascii_digit()),
                "Invalid config value: {} {key}.year = {:?}",
                path.display(),
                value
            );
        }
        if let Some(value) = licence_name {
            anyhow::ensure!(
                value.eq_ignore_ascii_case("LICENCE") || value.eq_ignore_ascii_case("LICENSE"),
                "Invalid config value: {} {key}.licence_file_name = {:?}",
                path.display(),
                value
            );
        }
        Ok(())
    }

    pub fn load_project_with_root(root: &std::path::Path) -> Result<(Self, PathBuf)> {
        let dir = std::env::var_os("PRJ_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(".config"))
            .join("licencify");
        Self::load_project_with_paths(root, &Self::global_path()?, &dir)
    }

    fn load_project_with_paths(
        root: &std::path::Path,
        global_path: &Path,
        dir: &Path,
    ) -> Result<(Self, PathBuf)> {
        let global = Self::read_config(global_path, true)?.unwrap_or_default();
        let shared = Self::read_config(&dir.join("config.toml"), false)?.unwrap_or_default();
        let local = Self::read_config(&dir.join("config.local.toml"), false)?.unwrap_or_default();
        Ok((merge(merge(global, shared), local), root.to_path_buf()))
    }

    pub fn load_effective(subdir: Option<&str>) -> Result<Self> {
        let (mut config, _) = Self::load_project_with_root(&Self::project_root()?)?;
        if let Some(path) = subdir {
            if let Some(entry) = config.subdirs.as_ref().and_then(|entries| {
                entries
                    .iter()
                    .filter(|entry| {
                        path == entry.path
                            || (path.len() > entry.path.len()
                                && path.as_bytes().get(entry.path.len()) == Some(&b'/')
                                && path.starts_with(&entry.path))
                    })
                    .max_by_key(|entry| entry.path.len())
            }) {
                config.default.author = entry.author.clone().or(config.default.author);
                config.default.company = entry.company.clone().or(config.default.company);
                config.default.email = entry.email.clone().or(config.default.email);
                config.default.license = entry.license.clone().or(config.default.license);
                config.default.format = entry.format.clone().or(config.default.format);
                config.default.year = entry.year.clone().or(config.default.year);
                config.default.licence_name =
                    entry.licence_name.clone().or(config.default.licence_name);
                config.default.additional_licences = entry
                    .additional_licences
                    .clone()
                    .or(config.default.additional_licences);
            }
        }
        Ok(config)
    }
    pub fn save_to_path(&self, path: &std::path::Path) -> Result<()> {
        let fs = global_fs();
        if let Some(parent) = path.parent() {
            fs.create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        let doc: toml_edit::DocumentMut = toml::to_string_pretty(self)?
            .parse()
            .context("Failed to parse serialized config")?;
        let mut output = format!("#:schema {}\n\n", Self::schema_path()?.display());
        output.push_str(&doc.to_string());
        fs.write(path, &output)
            .with_context(|| format!("Failed to write config file: {}", path.display()))
    }

    pub fn update_project_defaults(license: &str, author: &str, format: &str) -> Result<bool> {
        let path = Self::project_path()?;
        if !global_fs().exists(&path) {
            return Ok(false);
        }
        let Some(text) = global_fs().read_to_string(&path) else {
            anyhow::bail!("Failed to read project config: {}", path.display());
        };
        let mut config: Self = toml::from_str(&text)
            .with_context(|| format!("Failed to parse config {}", path.display()))?;
        config.default.license = Some(license.to_string());
        config.default.author = Some(author.to_string());
        config.default.format = Some(format.to_string());
        config.save_to_path(&path)?;
        Ok(true)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to_path(&Self::global_path()?)
    }

    pub fn licence_name_setting(&self) -> Option<&str> {
        self.default.licence_name.as_deref()
    }

    pub fn find_custom_template(&self, spdx_id: &str, format: &str) -> Option<(String, String)> {
        let fs = global_fs();
        let paths = self.template.as_ref()?.paths.as_ref()?;
        let filename = format!("{}.{}", spdx_id, format);
        for dir in paths {
            if let Some(text) = fs.read_to_string(&std::path::Path::new(dir).join(&filename)) {
                return Some((text, format!("custom ({dir})")));
            }
        }
        None
    }

    pub fn schema_json() -> Result<String> {
        serde_json::to_string_pretty(&schemars::schema_for!(Config))
            .context("Failed to serialize JSON schema")
    }

    pub fn schema_path() -> Result<PathBuf> {
        Ok(Self::global_path()?.with_file_name("licencify-schema.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{FsGuard, MemFs};
    use std::sync::Arc;

    #[test]
    fn schema_json_is_valid_json() {
        let _: serde_json::Value = serde_json::from_str(&Config::schema_json().unwrap()).unwrap();
    }

    #[test]
    fn project_root_precedence_and_containment() {
        let cwd = Path::new("/work/repo/sub");
        assert_eq!(
            Config::select_project_root(
                cwd,
                Some(PathBuf::from("/work/repo")),
                Some(PathBuf::from("/git-root")),
                Some(PathBuf::from("/config-root")),
            )
            .unwrap(),
            PathBuf::from("/work/repo")
        );
        assert_eq!(
            Config::select_project_root(
                cwd,
                None,
                Some(PathBuf::from("/git-root")),
                Some(PathBuf::from("/config-root")),
            )
            .unwrap(),
            PathBuf::from("/git-root")
        );
        assert_eq!(
            Config::select_project_root(cwd, None, None, Some(PathBuf::from("/config-root")))
                .unwrap(),
            PathBuf::from("/config-root")
        );
        assert_eq!(
            Config::select_project_root(cwd, None, None, None).unwrap(),
            cwd.to_path_buf()
        );
        assert!(
            Config::select_project_root(cwd, Some(PathBuf::from("/outside")), None, None,).is_err()
        );
        assert!(
            Config::select_project_root(cwd, Some(PathBuf::from("relative")), None, None,).is_err()
        );
    }

    #[test]
    fn config_default_is_empty() {
        assert!(Config::default().default.author.is_none());
    }

    #[test]
    fn project_load_merges_optional_global_then_shared() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        let root = PathBuf::from("/project");
        fs.write_file(
            "/user-config/licencify/config.toml",
            "[default]\nauthor = 'Global'\nyear = '2020'\n",
        );
        fs.write_file(
            "/project-config/licencify/config.toml",
            "[default]\nauthor = 'Project'\n",
        );
        let (config, _) = Config::load_project_with_paths(
            &root,
            Path::new("/user-config/licencify/config.toml"),
            Path::new("/project-config/licencify"),
        )
        .unwrap();
        assert_eq!(config.default.author.as_deref(), Some("Project"));
        assert_eq!(config.default.year.as_deref(), Some("2020"));
    }

    #[test]
    fn invalid_known_config_values_are_reported() {
        let invalid = [
            (
                "[default]\nlicense = 'not-an-spdx-id'\n",
                "default].licence",
            ),
            ("[default]\nformat = 'pdf'\n", "default].format"),
            (
                "[default]\nlicence_file_name = 'LICENSE.txt'\n",
                "default].licence_file_name",
            ),
            (
                "[default]\nadditional_licences = ['not-an-spdx-id']\n",
                "default].additional-licences",
            ),
            (
                "[[subdirs]]\npath = 'docs'\nformat = 'pdf'\n",
                "subdirs[0].format",
            ),
        ];
        for (text, key) in invalid {
            let config: Config = toml::from_str(text).unwrap();
            let error = Config::validate(&config, Path::new("test.toml")).unwrap_err();
            assert!(error.to_string().contains(key), "{error}");
        }

        let config: Config = toml::from_str(
            "[default]\nlicense = 'MIT'\nformat = 'md'\nlicence_file_name = 'license'\nadditional_licences = ['Apache-2.0']\n",
        )
        .unwrap();
        assert!(Config::validate(&config, Path::new("test.toml")).is_ok());
    }

    #[test]
    fn legacy_root_config_is_not_loaded() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        fs.write_file("/project/licencify.toml", "[default]\nauthor = 'Legacy'\n");
        let (config, _) = Config::load_project_with_paths(
            Path::new("/project"),
            Path::new("/empty-global-config"),
            Path::new("/project/.config/licencify"),
        )
        .unwrap();
        assert!(config.default.author.is_none());
    }

    #[test]
    fn merge_prefers_overriding_values() {
        let mut base = Config::default();
        base.default.author = Some("Base".into());
        let mut over = Config::default();
        over.default.author = Some("Override".into());
        assert_eq!(
            merge(base, over).default.author.as_deref(),
            Some("Override")
        );
    }

    #[test]
    fn licence_name_setting_returns_none_when_not_configured() {
        assert!(Config::default().licence_name_setting().is_none());
    }

    #[test]
    fn subdirs_serde_roundtrip() {
        let config = Config {
            subdirs: Some(vec![SubdirConfig {
                path: "docs".into(),
                author: Some("Alice".into()),
                ..Default::default()
            }]),
            ..Config::default()
        };
        let parsed: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(parsed.subdirs.unwrap()[0].author.as_deref(), Some("Alice"));
    }
}
