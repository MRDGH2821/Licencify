use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::io::{IsTerminal, Write};
use std::path::{Component, Path, PathBuf};

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

    /// README update override for this sub-directory
    #[schemars(description = "README update override")]
    #[serde(default)]
    pub update_readme: Option<bool>,

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

/// Where add and update persist the selected licence, author, and format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigWriteTarget {
    /// Shared or local `[default]` table.
    SharedDefaults,
    /// Exact-path `[[subdirs]]` entry for the current directory.
    ExactSubdir,
}

/// Invocation options read by configuration loading and writes.
#[derive(Clone, Copy, Debug, Default)]
pub struct CliContext {
    pub verbose: bool,
    pub yes: bool,
    pub config_target: Option<ConfigWriteTarget>,
}

/// File and rule that supplied one effective setting. Values are omitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSource {
    pub field: String,
    pub origin: String,
}

thread_local! {
    static CLI_CONTEXT: RefCell<CliContext> = const { RefCell::new(CliContext {
        verbose: false,
        yes: false,
        config_target: None,
    }) };
}

pub fn set_cli_context(context: CliContext) {
    CLI_CONTEXT.with(|slot| *slot.borrow_mut() = context);
}

fn cli_context() -> CliContext {
    CLI_CONTEXT.with(|slot| *slot.borrow())
}

pub fn format_setting_sources(sources: &[FieldSource]) -> Vec<String> {
    const ORDER: &[&str] = &[
        "author",
        "company",
        "email",
        "licence",
        "format",
        "year",
        "licence_file_name",
        "update_readme",
        "additional-licences",
    ];
    let mut lines = Vec::new();
    for field in ORDER {
        if let Some(source) = sources.iter().find(|source| source.field == *field) {
            lines.push(format!("{}: {}", source.field, source.origin));
        }
    }
    lines
}

fn normalize_subdir_path(raw: &str) -> Result<String> {
    let raw = raw.replace('\\', "/");
    let path = Path::new(&raw);
    if path.is_absolute() {
        anyhow::bail!("subdir path {raw:?} escapes the project root");
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    anyhow::bail!("subdir path {raw:?} escapes the project root");
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!("subdir path {raw:?} escapes the project root");
            }
        }
    }
    let normalized = parts.join("/");
    if normalized.is_empty() {
        anyhow::bail!("subdir path {raw:?} escapes the project root");
    }
    Ok(normalized)
}

fn relative_within_root(root: &Path, cwd: &Path) -> Result<String> {
    if let Some(relative) = strip_relative(root, cwd) {
        return Ok(relative);
    }
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let cwd = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    strip_relative(&root, &cwd).context("Current directory is outside the project root")
}

fn strip_relative(root: &Path, cwd: &Path) -> Option<String> {
    let relative = cwd.strip_prefix(root).ok()?;
    if relative.as_os_str().is_empty() {
        return Some(String::new());
    }
    let raw = relative.to_string_lossy().replace('\\', "/");
    normalize_subdir_path(&raw).ok()
}

fn child_config_warnings(root: &Path, cwd: &Path, root_config: &Path) -> Vec<String> {
    let Ok(relative) = cwd.strip_prefix(root) else {
        return Vec::new();
    };
    if relative.as_os_str().is_empty() {
        return Vec::new();
    }
    let mut warnings = Vec::new();
    let mut dir = root.to_path_buf();
    for component in relative.components() {
        dir.push(component);
        for name in [
            ".config/licencify/config.toml",
            ".config/licencify/config.local.toml",
            "licencify.toml",
            ".licencify.toml",
        ] {
            let path = dir.join(name);
            if global_fs().exists(&path) {
                warnings.push(format!(
                    "Warning: ignored child config {}; merge it into the root shared config {}",
                    path.display(),
                    root_config.display()
                ));
            }
        }
    }
    warnings
}

fn path_matches(entry: &str, relative: &str) -> bool {
    relative == entry || relative.starts_with(&format!("{entry}/"))
}

fn matching_subdirs<'a>(
    entries: Option<&'a [SubdirConfig]>,
    relative: &str,
) -> Vec<&'a SubdirConfig> {
    let mut matched: Vec<_> = entries
        .unwrap_or(&[])
        .iter()
        .filter(|entry| path_matches(&entry.path, relative))
        .collect();
    matched.sort_by_key(|entry| {
        entry
            .path
            .split('/')
            .filter(|part| !part.is_empty())
            .count()
    });
    matched
}

fn set_field<T: Clone>(
    slot: &mut Option<T>,
    incoming: &Option<T>,
    field: &str,
    origin: &str,
    sources: &mut Vec<FieldSource>,
) {
    if incoming.is_some() {
        *slot = incoming.clone();
        sources.retain(|source| source.field != field);
        sources.push(FieldSource {
            field: field.to_string(),
            origin: origin.to_string(),
        });
    }
}

fn overlay_default(
    base: &mut DefaultConfig,
    over: &DefaultConfig,
    origin: &str,
    sources: &mut Vec<FieldSource>,
) {
    set_field(&mut base.author, &over.author, "author", origin, sources);
    set_field(&mut base.company, &over.company, "company", origin, sources);
    set_field(&mut base.email, &over.email, "email", origin, sources);
    set_field(&mut base.license, &over.license, "licence", origin, sources);
    set_field(&mut base.format, &over.format, "format", origin, sources);
    set_field(&mut base.year, &over.year, "year", origin, sources);
    set_field(
        &mut base.licence_name,
        &over.licence_name,
        "licence_file_name",
        origin,
        sources,
    );
    set_field(
        &mut base.update_readme,
        &over.update_readme,
        "update_readme",
        origin,
        sources,
    );
    set_field(
        &mut base.additional_licences,
        &over.additional_licences,
        "additional-licences",
        origin,
        sources,
    );
}

fn overlay_subdir(
    base: &mut DefaultConfig,
    entry: &SubdirConfig,
    origin: &str,
    sources: &mut Vec<FieldSource>,
) {
    set_field(&mut base.author, &entry.author, "author", origin, sources);
    set_field(
        &mut base.company,
        &entry.company,
        "company",
        origin,
        sources,
    );
    set_field(&mut base.email, &entry.email, "email", origin, sources);
    set_field(
        &mut base.license,
        &entry.license,
        "licence",
        origin,
        sources,
    );
    set_field(&mut base.format, &entry.format, "format", origin, sources);
    set_field(&mut base.year, &entry.year, "year", origin, sources);
    set_field(
        &mut base.licence_name,
        &entry.licence_name,
        "licence_file_name",
        origin,
        sources,
    );
    set_field(
        &mut base.update_readme,
        &entry.update_readme,
        "update_readme",
        origin,
        sources,
    );
    set_field(
        &mut base.additional_licences,
        &entry.additional_licences,
        "additional-licences",
        origin,
        sources,
    );
}

fn concatenate_subdirs(shared: &Config, local: &Config) -> Option<Vec<SubdirConfig>> {
    let mut entries = Vec::new();
    if let Some(items) = &shared.subdirs {
        entries.extend(items.clone());
    }
    if let Some(items) = &local.subdirs {
        entries.extend(items.clone());
    }
    (!entries.is_empty()).then_some(entries)
}

fn resolve_layers(
    global: &Config,
    shared: &Config,
    local: &Config,
    relative: Option<&str>,
) -> Result<(Config, Vec<FieldSource>)> {
    let mut sources = Vec::new();
    let mut default = DefaultConfig::default();
    overlay_default(
        &mut default,
        &global.default,
        "global config [default]",
        &mut sources,
    );
    overlay_default(
        &mut default,
        &shared.default,
        "shared config.toml [default]",
        &mut sources,
    );
    if let Some(relative) = relative {
        for entry in matching_subdirs(shared.subdirs.as_deref(), relative) {
            let origin = format!("shared config.toml [[subdirs]] {}", entry.path);
            overlay_subdir(&mut default, entry, &origin, &mut sources);
        }
    }
    overlay_default(
        &mut default,
        &local.default,
        "local config.local.toml [default]",
        &mut sources,
    );
    if let Some(relative) = relative {
        for entry in matching_subdirs(local.subdirs.as_deref(), relative) {
            let origin = format!("local config.local.toml [[subdirs]] {}", entry.path);
            overlay_subdir(&mut default, entry, &origin, &mut sources);
        }
    }
    let mut config = merge(merge(global.clone(), shared.clone()), local.clone());
    config.default = default;
    config.subdirs = concatenate_subdirs(shared, local);
    ensure_no_proprietary_mix(&config)?;
    Ok((config, sources))
}

fn ensure_no_proprietary_mix(config: &Config) -> Result<()> {
    let extras = config.default.additional_licences.as_deref().unwrap_or(&[]);
    let primary_proprietary = config
        .default
        .license
        .as_deref()
        .is_some_and(|id| id.eq_ignore_ascii_case("proprietary"));
    let extra_proprietary = extras
        .iter()
        .any(|id| id.eq_ignore_ascii_case("proprietary"));
    if extra_proprietary || (primary_proprietary && !extras.is_empty()) {
        anyhow::bail!(
            "Invalid effective configuration: proprietary cannot be combined with open-source additional licences"
        );
    }
    Ok(())
}

fn choose_write_target(
    relative: &str,
    explicit: Option<ConfigWriteTarget>,
    noninteractive: bool,
    prompted: Option<ConfigWriteTarget>,
) -> Result<ConfigWriteTarget> {
    if relative.is_empty() {
        if explicit == Some(ConfigWriteTarget::ExactSubdir) {
            anyhow::bail!("--config-target subdir requires a project subdirectory");
        }
        return Ok(ConfigWriteTarget::SharedDefaults);
    }
    if let Some(target) = explicit {
        return Ok(target);
    }
    if noninteractive {
        return Ok(ConfigWriteTarget::ExactSubdir);
    }
    prompted.context("Select shared defaults or an exact subdir entry")
}

fn prompt_config_target(relative: &str) -> Result<ConfigWriteTarget> {
    println!("Save licence settings for `{relative}`:");
    println!("  [1] shared project defaults");
    println!("  [2] exact subdir entry");
    print!("Choice [2]: ");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    match input.trim() {
        "" | "2" | "subdir" => Ok(ConfigWriteTarget::ExactSubdir),
        "1" | "shared" => Ok(ConfigWriteTarget::SharedDefaults),
        other => {
            anyhow::bail!("Unrecognized config target `{other}`; choose 1 (shared) or 2 (subdir)")
        }
    }
}

fn masks_saved_fields(license: Option<&str>, author: Option<&str>, format: Option<&str>) -> bool {
    license.is_some() || author.is_some() || format.is_some()
}

fn local_mask_target(local: &Config, relative: &str) -> Option<ConfigWriteTarget> {
    let mut winner = None;
    if masks_saved_fields(
        local.default.license.as_deref(),
        local.default.author.as_deref(),
        local.default.format.as_deref(),
    ) {
        winner = Some(ConfigWriteTarget::SharedDefaults);
    }
    if !relative.is_empty() {
        for entry in matching_subdirs(local.subdirs.as_deref(), relative) {
            if masks_saved_fields(
                entry.license.as_deref(),
                entry.author.as_deref(),
                entry.format.as_deref(),
            ) {
                winner = Some(ConfigWriteTarget::ExactSubdir);
            }
        }
    }
    winner
}

fn local_mask_path(local: &Config, relative: &str) -> Option<String> {
    let mut path = None;
    if !relative.is_empty() {
        for entry in matching_subdirs(local.subdirs.as_deref(), relative) {
            if masks_saved_fields(
                entry.license.as_deref(),
                entry.author.as_deref(),
                entry.format.as_deref(),
            ) {
                path = Some(entry.path.clone());
            }
        }
    }
    path
}

fn refuse_child_config_write(root: &Path, path: &Path) -> Result<()> {
    let Ok(relative) = path.strip_prefix(root) else {
        return Ok(());
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    if relative != ".config/licencify/config.toml"
        && relative != ".config/licencify/config.local.toml"
        && (relative.contains(".config/licencify/")
            || relative.ends_with("licencify.toml")
            || relative.ends_with(".licencify.toml"))
    {
        anyhow::bail!("Refusing to write child config {}", path.display());
    }
    Ok(())
}

fn set_preserved_string(table: &mut toml_edit::Table, key: &str, new_value: &str) {
    if let Some(toml_edit::Item::Value(existing)) = table.get_mut(key) {
        let decor = existing.decor().clone();
        let mut value = toml_edit::Value::from(new_value.to_owned());
        *value.decor_mut() = decor;
        *existing = value;
        return;
    }
    table.insert(key, toml_edit::value(new_value.to_owned()));
}

fn set_saved_fields(table: &mut toml_edit::Table, license: &str, author: &str, format: &str) {
    if table.contains_key("license") && !table.contains_key("licence") {
        set_preserved_string(table, "license", license);
    } else {
        set_preserved_string(table, "licence", license);
    }
    set_preserved_string(table, "author", author);
    set_preserved_string(table, "format", format);
}

fn edit_config_text(
    text: &str,
    slot: ConfigWriteTarget,
    relative: &str,
    subdir_path: Option<&str>,
    license: &str,
    author: &str,
    format: &str,
) -> Result<String> {
    let mut doc = if text.trim().is_empty() {
        toml_edit::DocumentMut::new()
    } else {
        text.parse::<toml_edit::DocumentMut>()
            .context("Failed to parse project config")?
    };
    match slot {
        ConfigWriteTarget::SharedDefaults => {
            if !doc.contains_key("default") {
                doc.insert("default", toml_edit::Item::Table(toml_edit::Table::new()));
            }
            let table = doc
                .get_mut("default")
                .and_then(toml_edit::Item::as_table_mut)
                .context("`[default]` must be a table")?;
            set_saved_fields(table, license, author, format);
        }
        ConfigWriteTarget::ExactSubdir => {
            let path = subdir_path.unwrap_or(relative);
            anyhow::ensure!(
                !path.is_empty(),
                "Cannot save an exact subdir entry at the project root"
            );
            if !doc.contains_key("subdirs") {
                doc.insert(
                    "subdirs",
                    toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()),
                );
            }
            let tables = doc
                .get_mut("subdirs")
                .and_then(toml_edit::Item::as_array_of_tables_mut)
                .context("`subdirs` must be an array of tables")?;
            let mut index = None;
            for position in 0..tables.len() {
                let Some(table) = tables.get(position) else {
                    continue;
                };
                let Some(existing) = table.get("path").and_then(toml_edit::Item::as_str) else {
                    continue;
                };
                if normalize_subdir_path(existing).ok().as_deref() == Some(path) {
                    index = Some(position);
                }
            }
            if let Some(position) = index {
                let table = tables
                    .get_mut(position)
                    .context("Missing subdirectory table")?;
                set_saved_fields(table, license, author, format);
            } else {
                let mut table = toml_edit::Table::new();
                table.insert("path", toml_edit::value(path.to_owned()));
                set_saved_fields(&mut table, license, author, format);
                tables.push(table);
            }
        }
    }
    Ok(doc.to_string())
}

fn write_project_selection(
    shared_path: &Path,
    local_path: &Path,
    root: &Path,
    relative: &str,
    target: ConfigWriteTarget,
    license: &str,
    author: &str,
    format: &str,
) -> Result<bool> {
    refuse_child_config_write(root, shared_path)?;
    refuse_child_config_write(root, local_path)?;
    let fs = global_fs();
    let local_config = Config::read_config(local_path, false)?.unwrap_or_default();
    let mask = local_mask_target(&local_config, relative);
    let (destination, slot, subdir_path, masked) = if let Some(mask_target) = mask {
        let subdir_path = local_mask_path(&local_config, relative);
        (local_path, mask_target, subdir_path, true)
    } else {
        let subdir_path = (target == ConfigWriteTarget::ExactSubdir && !relative.is_empty())
            .then(|| relative.to_string());
        (shared_path, target, subdir_path, false)
    };
    if masked {
        eprintln!(
            "Warning: local override masks saved settings; updating {} instead",
            destination.display()
        );
    }
    refuse_child_config_write(root, destination)?;
    if slot == ConfigWriteTarget::SharedDefaults && !fs.exists(destination) {
        return Ok(false);
    }
    let existing = fs.read_to_string(destination);
    let created = existing.is_none();
    let edited = edit_config_text(
        existing.as_deref().unwrap_or(""),
        slot,
        relative,
        subdir_path.as_deref(),
        license,
        author,
        format,
    )?;
    let output = if created {
        format!("#:schema {}\n\n{edited}", Config::schema_path()?.display())
    } else {
        edited
    };
    if let Some(parent) = destination.parent() {
        fs.create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    fs.write(destination, &output)
        .with_context(|| format!("Failed to write config file: {}", destination.display()))?;
    Ok(true)
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
                                "update_readme",
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
        let mut config = if global {
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
        if let Some(entries) = &mut config.subdirs {
            for entry in entries.iter_mut() {
                entry.path = normalize_subdir_path(&entry.path)?;
            }
        }
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
                normalize_subdir_path(&entry.path).map_err(|error| {
                    anyhow::anyhow!(
                        "Invalid config value: {} subdirs[{index}].path = {:?}: {error}",
                        path.display(),
                        entry.path
                    )
                })?;
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

    fn resolve_from_dir(
        global_path: &Path,
        dir: &Path,
        relative: Option<&str>,
    ) -> Result<(Self, Vec<FieldSource>)> {
        let global = Self::read_config(global_path, true)?.unwrap_or_default();
        let shared = Self::read_config(&dir.join("config.toml"), false)?.unwrap_or_default();
        let local = Self::read_config(&dir.join("config.local.toml"), false)?.unwrap_or_default();
        resolve_layers(&global, &shared, &local, relative)
    }

    pub fn load_effective(subdir: Option<&str>) -> Result<Self> {
        let root = Self::project_root()?;
        let cwd = std::env::current_dir().context("Could not determine current directory")?;
        for warning in child_config_warnings(&root, &cwd, &Self::project_path()?) {
            eprintln!("{warning}");
        }
        let relative = match subdir {
            Some(path) => {
                let normalized =
                    normalize_subdir_path(path).context("Invalid subdirectory override path")?;
                Some(normalized)
            }
            None => {
                let relative = relative_within_root(&root, &cwd)?;
                (!relative.is_empty()).then_some(relative)
            }
        };
        let (config, sources) = Self::resolve_from_dir(
            &Self::global_path()?,
            &Self::project_config_dir()?,
            relative.as_deref(),
        )?;
        if cli_context().verbose {
            for line in format_setting_sources(&sources) {
                eprintln!("setting {line}");
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
        let root = Self::project_root()?;
        let cwd = std::env::current_dir().context("Could not determine current directory")?;
        let relative = relative_within_root(&root, &cwd)?;
        let context = cli_context();
        let target = if let Some(target) = context.config_target {
            choose_write_target(&relative, Some(target), true, None)?
        } else if relative.is_empty() || context.yes || !std::io::stdin().is_terminal() {
            choose_write_target(&relative, None, true, None)?
        } else {
            prompt_config_target(&relative)?
        };
        write_project_selection(
            &Self::project_path()?,
            &Self::local_path()?,
            &root,
            &relative,
            target,
            license,
            author,
            format,
        )
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
    use crate::fs::{Fs, FsGuard, MemFs};
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

    fn sample_layers() -> (Config, Config, Config) {
        let mut global = Config::default();
        global.default.author = Some("Global".into());
        global.default.additional_licences = Some(vec!["MIT".into()]);

        let mut shared = Config::default();
        shared.default.company = Some("SharedCo".into());
        shared.subdirs = Some(vec![
            SubdirConfig {
                path: "docs".into(),
                author: Some("Alice".into()),
                additional_licences: Some(vec!["Apache-2.0".into()]),
                ..Default::default()
            },
            SubdirConfig {
                path: "docs/api".into(),
                license: Some("ISC".into()),
                format: Some("txt".into()),
                ..Default::default()
            },
        ]);

        let mut local = Config::default();
        local.default.year = Some("2024".into());
        local.default.license = Some("Apache-2.0".into());
        local.subdirs = Some(vec![
            SubdirConfig {
                path: "docs".into(),
                format: Some("html".into()),
                ..Default::default()
            },
            SubdirConfig {
                path: "docs/api".into(),
                additional_licences: Some(vec![]),
                ..Default::default()
            },
        ]);
        (global, shared, local)
    }

    #[test]
    fn layered_subdirs_resolve_field_by_field_and_empty_list_clears() {
        let (global, shared, local) = sample_layers();
        let (config, sources) =
            resolve_layers(&global, &shared, &local, Some("docs/api/extra")).unwrap();
        assert_eq!(config.default.author.as_deref(), Some("Alice"));
        assert_eq!(config.default.company.as_deref(), Some("SharedCo"));
        assert_eq!(config.default.year.as_deref(), Some("2024"));
        assert_eq!(config.default.license.as_deref(), Some("Apache-2.0"));
        assert_eq!(config.default.format.as_deref(), Some("html"));
        assert_eq!(config.default.additional_licences, Some(vec![]));

        let lines = format_setting_sources(&sources).join("\n");
        assert!(lines.contains("author: shared config.toml [[subdirs]] docs"));
        assert!(lines.contains("licence: local config.local.toml [default]"));
        assert!(
            lines.contains("additional-licences: local config.local.toml [[subdirs]] docs/api")
        );
        assert!(!lines.contains("Alice"));
        assert!(!lines.contains("Apache-2.0"));
        assert!(!lines.contains("SharedCo"));

        let (boundary, _) =
            resolve_layers(&global, &shared, &local, Some("docs/api-extra")).unwrap();
        assert_eq!(boundary.default.license.as_deref(), Some("Apache-2.0"));
        assert_eq!(
            boundary.default.additional_licences,
            Some(vec!["Apache-2.0".into()])
        );
        assert_ne!(boundary.default.license.as_deref(), Some("ISC"));
    }

    #[test]
    fn empty_additional_licences_in_toml_is_present() {
        let config: Config =
            toml::from_str("[[subdirs]]\npath = 'docs/api'\nadditional-licences = []\n").unwrap();
        assert_eq!(
            config.subdirs.unwrap()[0].additional_licences.as_deref(),
            Some(&[][..])
        );
    }

    #[test]
    fn subdir_paths_are_normalized_and_cannot_escape() {
        for path in ["../secret", "/tmp/outside", "docs/../../etc", "."] {
            let config: Config =
                toml::from_str(&format!("[[subdirs]]\npath = '{path}'\n")).unwrap();
            let error = Config::validate(&config, Path::new("test.toml")).unwrap_err();
            assert!(
                error.to_string().contains("subdirs[0].path"),
                "{path}: {error}"
            );
            assert!(
                error.to_string().contains("escapes the project root"),
                "{error}"
            );
        }

        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        fs.write_file(
            "/project/.config/licencify/config.toml",
            "[[subdirs]]\npath = './docs/api'\nformat = 'txt'\n",
        );
        let (config, _) = Config::resolve_from_dir(
            Path::new("/missing-global"),
            Path::new("/project/.config/licencify"),
            Some("docs/api"),
        )
        .unwrap();
        assert_eq!(config.default.format.as_deref(), Some("txt"));
        assert_eq!(config.subdirs.unwrap()[0].path, "docs/api");
    }

    #[test]
    fn child_config_files_are_ignored_with_a_warning() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        fs.write_file(
            "/project/.config/licencify/config.toml",
            "[default]\nauthor = 'Root'\n",
        );
        fs.write_file(
            "/project/docs/.config/licencify/config.toml",
            "[default]\nauthor = 'Child'\n",
        );
        fs.write_file("/project/docs/api/.licencify.toml", "author = 'Nested'\n");
        let warnings = child_config_warnings(
            Path::new("/project"),
            Path::new("/project/docs/api"),
            Path::new("/project/.config/licencify/config.toml"),
        );
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("ignored child config"));
        assert!(warnings[0].contains("docs/.config/licencify/config.toml"));
        assert!(warnings[0].contains("merge it into the root shared config"));
        assert!(warnings[1].contains(".licencify.toml"));
        let root_only = child_config_warnings(
            Path::new("/project"),
            Path::new("/project"),
            Path::new("/project/.config/licencify/config.toml"),
        );
        assert!(root_only.is_empty());
        let (config, _) = Config::resolve_from_dir(
            Path::new("/missing-global"),
            Path::new("/project/.config/licencify"),
            Some("docs/api"),
        )
        .unwrap();
        assert_eq!(config.default.author.as_deref(), Some("Root"));
    }

    #[test]
    fn proprietary_mix_is_rejected_after_resolution() {
        let mut shared = Config::default();
        shared.default.license = Some("MIT".into());
        shared.default.additional_licences = Some(vec!["Apache-2.0".into()]);
        let mut local = Config::default();
        local.subdirs = Some(vec![SubdirConfig {
            path: "docs".into(),
            license: Some("proprietary".into()),
            ..Default::default()
        }]);
        let error = resolve_layers(&Config::default(), &shared, &local, Some("docs"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("proprietary"), "{error}");
    }

    #[test]
    fn config_target_selection_defaults_to_exact_subdir() {
        assert_eq!(
            choose_write_target("", None, true, None).unwrap(),
            ConfigWriteTarget::SharedDefaults
        );
        assert_eq!(
            choose_write_target("docs/api", None, true, None).unwrap(),
            ConfigWriteTarget::ExactSubdir
        );
        assert_eq!(
            choose_write_target(
                "docs/api",
                Some(ConfigWriteTarget::SharedDefaults),
                false,
                None
            )
            .unwrap(),
            ConfigWriteTarget::SharedDefaults
        );
        assert_eq!(
            choose_write_target(
                "docs/api",
                None,
                false,
                Some(ConfigWriteTarget::SharedDefaults)
            )
            .unwrap(),
            ConfigWriteTarget::SharedDefaults
        );
        assert!(choose_write_target("", Some(ConfigWriteTarget::ExactSubdir), true, None).is_err());
    }

    #[test]
    fn noninteractive_subdir_write_preserves_unrelated_fields_and_comments() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        let root = Path::new("/project");
        let shared = root.join(".config/licencify/config.toml");
        let local = root.join(".config/licencify/config.local.toml");
        fs.write_file(
            shared.as_path(),
            "# project header\n[default]\nauthor = \"Old\"\ncompany = \"KeepMe\" # company comment\nyear = \"1999\"\n",
        );
        let wrote = write_project_selection(
            &shared,
            &local,
            root,
            "docs/api",
            ConfigWriteTarget::ExactSubdir,
            "MIT",
            "Ada",
            "txt",
        )
        .unwrap();
        assert!(wrote);
        let text = fs.read_to_string(&shared).unwrap();
        assert!(text.contains("# project header"));
        assert!(text.contains("KeepMe"));
        assert!(text.contains("# company comment"));
        assert!(text.contains("1999"));
        assert!(text.contains("docs/api"));
        assert!(text.contains("Ada"));
        assert!(!fs.exists(&local));
    }

    #[test]
    fn explicit_shared_target_preserves_comments_and_local_mask_wins() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        let root = Path::new("/project");
        let shared = root.join(".config/licencify/config.toml");
        let local = root.join(".config/licencify/config.local.toml");
        fs.write_file(
            shared.as_path(),
            "# shared\n[default]\nlicence = \"MIT\"\nauthor = \"Old\"\nformat = \"txt\"\ncompany = \"KeepMe\" # stay\n",
        );
        fs.write_file(
            local.as_path(),
            "# local\n[default]\nauthor = \"Local\"\ncompany = \"Secret\"\n",
        );
        let wrote = write_project_selection(
            &shared,
            &local,
            root,
            "docs",
            ConfigWriteTarget::SharedDefaults,
            "Apache-2.0",
            "Ada",
            "html",
        )
        .unwrap();
        assert!(wrote);
        let shared_text = fs.read_to_string(&shared).unwrap();
        assert!(shared_text.contains("# shared"));
        assert!(shared_text.contains("Old"));
        assert!(shared_text.contains("KeepMe"));
        let local_text = fs.read_to_string(&local).unwrap();
        assert!(local_text.contains("# local"));
        assert!(local_text.contains("Secret"));
        assert!(local_text.contains("Ada"));
        assert!(local_text.contains("Apache-2.0"));
        assert!(local_text.contains("html"));
        assert!(!local_text.contains("Local\""));
    }

    #[test]
    fn child_config_write_is_refused() {
        let root = Path::new("/project");
        let child = root.join("docs/.config/licencify/config.toml");
        let error = refuse_child_config_write(root, &child).unwrap_err();
        assert!(error.to_string().contains("Refusing to write child config"));
        assert!(
            refuse_child_config_write(root, &root.join(".config/licencify/config.toml")).is_ok()
        );
    }

    #[test]
    fn missing_shared_config_is_created_for_exact_subdir() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        let root = Path::new("/project");
        let shared = root.join(".config/licencify/config.toml");
        let local = root.join(".config/licencify/config.local.toml");
        let wrote = write_project_selection(
            &shared,
            &local,
            root,
            "docs/api",
            ConfigWriteTarget::ExactSubdir,
            "MIT",
            "Ada",
            "txt",
        )
        .unwrap();
        assert!(wrote);
        let text = fs.read_to_string(&shared).unwrap();
        assert!(text.contains("#:schema "));
        assert!(text.contains("docs/api"));
        assert!(text.contains("MIT"));
        assert!(!fs.exists(&local));
    }

    #[test]
    fn local_subdir_mask_updates_the_winning_entry() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        crate::fs::set_global_fs(fs.clone());
        let root = Path::new("/project");
        let shared = root.join(".config/licencify/config.toml");
        let local = root.join(".config/licencify/config.local.toml");
        fs.write_file(shared.as_path(), "[default]\nauthor = \"Shared\"\n");
        fs.write_file(
            local.as_path(),
            "# keep local\n[[subdirs]]\npath = \"docs\"\nformat = \"txt\"\ncompany = \"LocalCo\"\n",
        );
        write_project_selection(
            &shared,
            &local,
            root,
            "docs/api",
            ConfigWriteTarget::ExactSubdir,
            "ISC",
            "Ada",
            "html",
        )
        .unwrap();
        let shared_text = fs.read_to_string(&shared).unwrap();
        assert_eq!(shared_text, "[default]\nauthor = \"Shared\"\n");
        let local_text = fs.read_to_string(&local).unwrap();
        assert!(local_text.contains("# keep local"));
        assert!(local_text.contains("LocalCo"));
        assert!(local_text.contains("ISC"));
        assert!(local_text.contains("Ada"));
        assert!(local_text.contains("html"));
        assert!(local_text.contains("path = \"docs\""));
    }
}
