use crate::{author, cache, licences, provider, template};

/// Resolve license template text using 3-tier chain:
/// 1. Built-in templates (instant, embedded in binary)
/// 2. Disk cache (fast, local)
/// 3. SPDX API (slowest, network)
pub fn resolve_template(spdx_id: &str) -> anyhow::Result<(String, String)> {
    let lower = spdx_id.to_lowercase();

    // Tier 1: Built-in templates
    if let Some(text) = licences::get(&lower) {
        return Ok((text.to_string(), "built-in".to_string()));
    }

    // Tier 2: Disk cache
    let cache = cache::LicenseCache::new()?;
    if let Some(text) = cache.get(&lower) {
        return Ok((text, "cached".to_string()));
    }

    // Tier 3: SPDX API
    let registry_dir = cache.dir().parent().unwrap().join("api");
    let prov = provider::LicenseProvider::with_api_cache(&registry_dir)?;
    match prov.fetch_detail(spdx_id) {
        Ok(detail) => {
            let _ = cache.put(&lower, &detail.license_text);
            Ok((detail.license_text, "SPDX API".to_string()))
        }
        Err(e) => {
            let supported = licences::supported_ids().join(", ");
            anyhow::bail!(
                "License '{}' not available as built-in template and \
                 could not be fetched from SPDX API: {}\n\
                 Built-in licenses: {}",
                spdx_id,
                e,
                supported
            );
        }
    }
}

/// Resolve year from CLI arg → config → current year.
pub fn resolve_year(cli_year: Option<String>) -> String {
    if let Some(year) = cli_year {
        return year;
    }
    if let Ok(cfg) = crate::config::Config::load() {
        if let Some(year) = cfg.year_override {
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
