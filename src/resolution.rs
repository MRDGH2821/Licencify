use crate::{author, licences, provider, template};

pub struct ResolvedTemplate {
    pub text: String,
    pub html: Option<String>,
    pub source: String,
}

/// Resolve license template text using 3-tier chain:
/// 1. API cache (fast, local JSON with licenseText)
/// 2. SPDX API (slowest, network)
/// 3. Built-in templates (instant, embedded in binary)
pub fn resolve_template(spdx_id: &str) -> anyhow::Result<ResolvedTemplate> {
    let prov = provider::LicenseProvider::load()?;

    // Tier 1: API cache
    if let Some(detail) = prov.get_cached(spdx_id) {
        return Ok(ResolvedTemplate {
            text: detail.license_text,
            html: detail.license_text_html,
            source: "cached".to_string(),
        });
    }

    // Tier 2: SPDX API
    if let Ok(detail) = prov.fetch_detail(spdx_id) {
        return Ok(ResolvedTemplate {
            text: detail.license_text,
            html: detail.license_text_html,
            source: "SPDX API".to_string(),
        });
    }

    // Tier 3: Built-in templates (plain text only, no HTML)
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
         and no built-in template exists.",
        spdx_id
    );
}

/// Resolve year from CLI arg → config → current year.
pub fn resolve_year(cli_year: Option<String>) -> String {
    if let Some(year) = cli_year {
        return year;
    }
    if let Ok(cfg) = crate::config::Config::load() {
        if let Some(year) = cfg.default.year {
            return year;
        }
    }
    template::default_year()
}

/// Resolve author using CLI arg → config → git config chain.
pub fn resolve_author(cli_author: Option<String>) -> anyhow::Result<String> {
    let resolvers: Vec<&dyn author::AuthorResolver> =
        vec![&author::ConfigAuthorResolver, &author::GitAuthorResolver];
    author::resolve_author(cli_author, &resolvers)
}
