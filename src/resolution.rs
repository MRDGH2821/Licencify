use crate::{
    author, config::Config, licence_name::LicenceName, licences, process::RealRunner,
    provider::LicenseProvider, template,
};

pub struct ResolvedTemplate {
    pub text: String,
    pub html: Option<String>,
    pub source: String,
}

/// Full resolved context for adding or updating a licence file.
pub struct ResolvedContext {
    pub author: String,
    pub year: String,
    pub licence_name: LicenceName,
    pub resolved: ResolvedTemplate,
}

/// Resolve licence name from config (if configured) or locale detection.
pub fn resolve_licence_name(config: Option<&Config>) -> LicenceName {
    LicenceName::resolve(config.and_then(|c| c.licence_name_setting()))
}

/// Resolve all context needed for add/update: author, year, licence name, template.
/// Avoids redundant `LicenseProvider::load()` by reusing the provider for both
/// `info()` and `resolve_template()`.
pub fn resolve_context(
    spdx_id: &str,
    cli_author: Option<String>,
    cli_year: Option<String>,
    config: Option<&Config>,
    provider: &LicenseProvider,
) -> anyhow::Result<ResolvedContext> {
    let author = resolve_author(cli_author, config)?;
    let year = resolve_year(cli_year, config);
    let licence_name = resolve_licence_name(config);
    let resolved = resolve_template(spdx_id, config, Some(provider))?;

    Ok(ResolvedContext {
        author,
        year,
        licence_name,
        resolved,
    })
}

/// Resolve license template text using 4-tier chain:
/// 1. API cache (fast, local JSON with licenseText)
/// 2. Custom template paths (from config [template].paths)
/// 3. SPDX API (network)
/// 4. Built-in templates (embedded in binary)
pub fn resolve_template(
    spdx_id: &str,
    config: Option<&Config>,
    provider: Option<&LicenseProvider>,
) -> anyhow::Result<ResolvedTemplate> {
    let prov = match provider {
        Some(p) => p,
        None => &LicenseProvider::load()?,
    };

    // Tier 1: API cache
    if let Some(detail) = prov.get_cached(spdx_id) {
        return Ok(ResolvedTemplate {
            text: detail.license_text,
            html: detail.license_text_html,
            source: "cached".to_string(),
        });
    }

    // Tier 2: Custom template paths from config [template].paths
    if let Some((text, html, source)) = config.and_then(|cfg| cfg.find_custom_template(spdx_id)) {
        return Ok(ResolvedTemplate { text, html, source });
    }

    // Tier 3: SPDX API
    if let Ok(detail) = prov.fetch_detail(spdx_id) {
        return Ok(ResolvedTemplate {
            text: detail.license_text,
            html: detail.license_text_html,
            source: "SPDX API".to_string(),
        });
    }

    // Tier 4: Built-in templates (plain text only, no HTML)
    let lower = spdx_id.to_lowercase();
    if let Some(text) = licences::get(&lower) {
        return Ok(ResolvedTemplate {
            text: text.to_string(),
            html: None,
            source: "built-in".to_string(),
        });
    }

    anyhow::bail!(
        "License '{}' not available. Not cached, not fetchable from SPDX API, \
         no custom template found, and no built-in template exists.",
        spdx_id
    );
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
