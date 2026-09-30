use crate::{
    author, cli::LicenseFormat, config::Config, licence_name::LicenceName, licences,
    process::RealRunner, provider::LicenseProvider, template,
};
use anyhow::Context;

pub struct ResolvedTemplate {
    pub text: String,
    pub source: String,
    pub format: LicenseFormat,
}

/// Full resolved context for adding or updating a licence file.
pub struct ResolvedContext {
    pub author: String,
    pub year: String,
    pub company: Option<String>,
    pub email: Option<String>,
    pub licence_name: LicenceName,
    pub resolved: ResolvedTemplate,
}

/// Resolve licence name from config (if configured) or locale detection.
pub fn resolve_licence_name(config: Option<&Config>) -> LicenceName {
    LicenceName::resolve(config.and_then(|c| c.licence_name_setting()))
}

/// Resolve all context needed for add/update: author, year, company, email, licence name, template.
/// Avoids redundant `LicenseProvider::load()` by reusing the provider for both
/// `info()` and `resolve_template()`.
pub fn resolve_context(
    spdx_id: &str,
    cli_author: Option<String>,
    cli_year: Option<String>,
    cli_company: Option<String>,
    cli_email: Option<String>,
    config: Option<&Config>,
    provider: &LicenseProvider,
    format: &LicenseFormat,
) -> anyhow::Result<ResolvedContext> {
    let author = resolve_author(cli_author, config)?;
    let year = resolve_year(cli_year, config);
    let company = cli_company.or_else(|| config.and_then(|c| c.default.company.clone()));
    let email = author::resolve_email(cli_email, config);
    let licence_name = resolve_licence_name(config);
    let resolved = resolve_template(spdx_id, config, Some(provider), format)?;

    Ok(ResolvedContext {
        author,
        year,
        company,
        email,
        licence_name,
        resolved,
    })
}

/// Resolve custom, SPDX detail, and bundled templates in source and format order.
pub fn resolve_template(
    spdx_id: &str,
    config: Option<&Config>,
    provider: Option<&LicenseProvider>,
    format: &LicenseFormat,
) -> anyhow::Result<ResolvedTemplate> {
    validate_template_id(spdx_id)?;
    let proprietary = spdx_id.eq_ignore_ascii_case("proprietary");
    let html_format = !matches!(format, LicenseFormat::Txt);
    let template_format = if html_format { "html" } else { "txt" };

    if let Some((text, source)) = find_custom_template(spdx_id, template_format, config)? {
        return Ok(resolved(text, source, format));
    }

    if proprietary {
        return resolve_bundled_or_fallback(spdx_id, format);
    }

    let owned_provider;
    let provider = match provider {
        Some(provider) => provider,
        None => {
            owned_provider = LicenseProvider::load()?;
            &owned_provider
        }
    };
    let cached = provider.get_cached(spdx_id);
    let was_cached = cached.is_some();
    let (detail, fetch_error) = match cached {
        Some(detail) => (Some(detail), None),
        None => match provider.fetch_detail(spdx_id) {
            Ok(detail) => (Some(detail), None),
            Err(error) => (None, Some(error)),
        },
    };

    if html_format {
        if let Some(text) = detail
            .as_ref()
            .and_then(|detail| detail.license_text_html.as_deref())
            .filter(|text| !text.trim().is_empty())
        {
            return Ok(resolved(
                text.to_string(),
                if was_cached { "cached" } else { "SPDX API" }.to_string(),
                format,
            ));
        }
    } else if let Some(text) = detail
        .as_ref()
        .map(|detail| detail.license_text.as_str())
        .filter(|text| !text.trim().is_empty())
    {
        return Ok(resolved(
            text.to_string(),
            if was_cached { "cached" } else { "SPDX API" }.to_string(),
            format,
        ));
    }

    if let Some(text) = bundled_template(spdx_id, template_format) {
        return Ok(resolved(text, "built-in".to_string(), format));
    }

    if html_format {
        eprintln!("Warning: no HTML template for {spdx_id}; writing plain text instead.");
        if let Some((text, source)) = find_custom_template(spdx_id, "txt", config)? {
            return Ok(resolved(text, source, &LicenseFormat::Txt));
        }
        if let Some(text) = detail
            .as_ref()
            .map(|detail| detail.license_text.as_str())
            .filter(|text| !text.trim().is_empty())
        {
            return Ok(resolved(
                text.to_string(),
                if was_cached { "cached" } else { "SPDX API" }.to_string(),
                &LicenseFormat::Txt,
            ));
        }
        if let Some(text) = bundled_template(spdx_id, "txt") {
            return Ok(resolved(text, "built-in".to_string(), &LicenseFormat::Txt));
        }
    }

    if let Some(error) = fetch_error {
        anyhow::bail!(
            "License '{spdx_id}' is unavailable locally and SPDX lookup failed: {error:#}"
        );
    }
    anyhow::bail!(
        "License '{spdx_id}' has no matching custom, cached, fetched, or bundled template."
    )
}

fn resolved(text: String, source: String, format: &LicenseFormat) -> ResolvedTemplate {
    ResolvedTemplate {
        text,
        source,
        format: format.clone(),
    }
}

fn validate_template_id(spdx_id: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !spdx_id.is_empty()
            && spdx_id != "."
            && spdx_id != ".."
            && spdx_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.'),
        "Invalid SPDX license identifier: '{spdx_id}'"
    );
    Ok(())
}

fn find_custom_template(
    spdx_id: &str,
    format: &str,
    config: Option<&Config>,
) -> anyhow::Result<Option<(String, String)>> {
    use crate::fs::global_fs;
    use std::path::PathBuf;

    let filename = if format == "html" {
        format!("{spdx_id}.html.tera")
    } else {
        format!("{spdx_id}.tera")
    };
    let fs = global_fs();
    let project_dir = project_template_dir()?;
    let global_dir = Config::global_path()?
        .parent()
        .context("Could not determine global template directory")?
        .join("templates");
    for (dir, source) in [(project_dir, "project"), (global_dir, "global")] {
        let path = dir.join(&filename);
        if fs.exists(&path) {
            let text = fs.read_to_string(&path).with_context(|| {
                format!("Failed to read {source} custom template {}", path.display())
            })?;
            return Ok(Some((text, format!("{source} custom ({})", dir.display()))));
        }
    }

    if let Some(paths) = config
        .and_then(|config| config.template.as_ref())
        .and_then(|template| template.paths.as_ref())
    {
        for dir in paths {
            let dir = PathBuf::from(dir);
            let path = dir.join(if format == "html" {
                format!("{spdx_id}.html.tera")
            } else {
                format!("{spdx_id}.tera")
            });
            if fs.exists(&path) {
                let text = fs.read_to_string(&path).with_context(|| {
                    format!("Failed to read custom template {}", path.display())
                })?;
                return Ok(Some((text, format!("custom ({})", dir.display()))));
            }
        }
    }
    Ok(None)
}

fn project_template_dir() -> anyhow::Result<std::path::PathBuf> {
    let root = Config::project_root()?;
    let config_home = std::env::var_os("PRJ_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join(".config"));
    Ok(config_home.join("licencify").join("templates"))
}

fn bundled_template(spdx_id: &str, format: &str) -> Option<String> {
    let templates = licences::get(&spdx_id.to_lowercase())?;
    let text = if format == "html" {
        templates.html
    } else {
        templates.txt
    };
    (!text.trim().is_empty()).then(|| text.to_string())
}

fn resolve_bundled_or_fallback(
    spdx_id: &str,
    format: &LicenseFormat,
) -> anyhow::Result<ResolvedTemplate> {
    if !matches!(format, LicenseFormat::Txt) {
        if let Some(text) = bundled_template(spdx_id, "html") {
            return Ok(resolved(text, "built-in".to_string(), format));
        }
        eprintln!("Warning: no HTML template for {spdx_id}; writing plain text instead.");
    }
    if let Some(text) = bundled_template(spdx_id, "txt") {
        return Ok(resolved(text, "built-in".to_string(), &LicenseFormat::Txt));
    }
    anyhow::bail!("No template is available for '{spdx_id}'")
}

/// Resolve year from CLI arg → config → current year.
pub fn resolve_year(cli_year: Option<String>, config: Option<&Config>) -> String {
    if let Some(year) = cli_year {
        return year;
    }
    if let Some(cfg) = config {
        if let Some(year) = &cfg.default.year {
            return year.clone();
        }
    }
    template::default_year()
}

/// Resolve author using CLI arg → config → git config chain.
pub fn resolve_author(
    cli_author: Option<String>,
    config: Option<&Config>,
) -> anyhow::Result<String> {
    let runner = RealRunner;
    let git_resolver = author::GitAuthorResolver { runner: &runner };
    let resolvers: Vec<&dyn author::AuthorResolver> =
        vec![&author::ConfigAuthorResolver, &git_resolver];
    author::resolve_author(cli_author, config, &resolvers)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, TemplateConfig},
        fs::{Fs, FsGuard, MemFs},
    };
    use std::{path::Path, sync::Arc};

    #[test]
    fn custom_template_precedes_cached_detail_without_fetching() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        fs.write_file(
            "/cache/MIT.json",
            r#"{"licenseId":"MIT","name":"MIT License","licenseText":"cached text","licenseTextHtml":"<p>cached html</p>"}"#,
        );
        fs.write_file("/project/MIT.tera", "project template");
        crate::fs::set_global_fs(fs);

        let provider = LicenseProvider::with_spdx_cache(Path::new("/cache")).unwrap();
        let config = Config {
            template: Some(TemplateConfig {
                paths: Some(vec!["/project".to_string()]),
            }),
            ..Config::default()
        };
        let resolved =
            resolve_template("MIT", Some(&config), Some(&provider), &LicenseFormat::Txt).unwrap();

        assert_eq!(resolved.text, "project template");
        assert_eq!(resolved.source, "custom (/project)");
    }
    #[test]
    fn project_then_global_templates_precede_cached_detail() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        let project_dir = project_template_dir().unwrap();
        let global_dir = dirs::config_dir()
            .unwrap()
            .join("licencify")
            .join("templates");
        if project_dir == global_dir {
            return;
        }
        fs.write_file(project_dir.join("MIT.tera"), "project wording");
        fs.write_file(global_dir.join("MIT.tera"), "global wording");
        fs.write_file(
            "/cache/MIT.json",
            r#"{"licenseId":"MIT","name":"MIT License","licenseText":"cached wording"}"#,
        );
        crate::fs::set_global_fs(fs.clone());
        let provider = LicenseProvider::with_spdx_cache(Path::new("/cache")).unwrap();

        let resolved = resolve_template("MIT", None, Some(&provider), &LicenseFormat::Txt).unwrap();
        assert_eq!(resolved.text, "project wording");

        fs.remove_dir_all(&project_dir).unwrap();
        let resolved = resolve_template("MIT", None, Some(&provider), &LicenseFormat::Txt).unwrap();
        assert_eq!(resolved.text, "global wording");
    }

    #[test]
    fn cached_text_is_used_without_a_network_fetch() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        fs.write_file(
            "/cache/Custom-1.json",
            r#"{"licenseId":"Custom-1","name":"Custom","licenseText":"cached wording"}"#,
        );
        crate::fs::set_global_fs(fs);
        let provider = LicenseProvider::with_spdx_cache(Path::new("/cache")).unwrap();

        let resolved =
            resolve_template("Custom-1", None, Some(&provider), &LicenseFormat::Txt).unwrap();

        assert_eq!(resolved.text, "cached wording");
        assert_eq!(resolved.format.to_string(), "txt");
    }

    #[test]
    fn missing_html_uses_cached_text_and_reports_text_format() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new());
        fs.write_file(
            "/cache/Custom-1.json",
            r#"{"licenseId":"Custom-1","name":"Custom","licenseText":"cached wording"}"#,
        );
        crate::fs::set_global_fs(fs);
        let provider = LicenseProvider::with_spdx_cache(Path::new("/cache")).unwrap();

        let resolved =
            resolve_template("Custom-1", None, Some(&provider), &LicenseFormat::Html).unwrap();

        assert_eq!(resolved.text, "cached wording");
        assert_eq!(resolved.format.to_string(), "txt");
    }

    #[test]
    fn proprietary_uses_bundled_notice_without_spdx_detail() {
        let _guard = FsGuard::new();
        crate::fs::set_global_fs(Arc::new(MemFs::new()));
        let resolved = resolve_template("proprietary", None, None, &LicenseFormat::Txt).unwrap();

        assert!(resolved.text.contains("All Rights Reserved"));
        assert_eq!(resolved.source, "built-in");
    }
}
