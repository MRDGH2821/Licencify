#![allow(dead_code)]
pub mod cache;
mod cli;
mod config;
mod licences;
mod license;
mod project;
mod registry;
mod spdx;
mod template;

use clap::Parser;
use cli::{CacheAction, Cli, Commands};

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add {
            spdx,
            author,
            year,
            yes,
        } => cmd_add(&spdx, author, year, yes),
        Commands::List {
            osi_only,
            fsf_only,
            limit,
        } => cmd_list(osi_only, fsf_only, limit),
        Commands::Search {
            query,
            osi_only,
            fsf_only,
        } => cmd_search(&query, osi_only, fsf_only),
        Commands::Detect => cmd_detect(),
        Commands::Update { spdx, author, year } => cmd_update(&spdx, author, year),
        Commands::Cache { action } => cmd_cache(action),
    }
}

/// Resolve author from CLI arg → config → git config → error.
fn resolve_author(cli_author: Option<String>) -> anyhow::Result<String> {
    // 1. CLI argument
    if let Some(author) = cli_author {
        return Ok(author);
    }

    // 2. Config file
    if let Ok(cfg) = config::Config::load() {
        if let Ok(author) = cfg.effective_author() {
            return Ok(author);
        }
    }

    // 3. Git config
    template::default_author()
}

/// Resolve year from CLI arg → config → current year.
fn resolve_year(cli_year: Option<String>) -> String {
    // 1. CLI argument
    if let Some(year) = cli_year {
        return year;
    }

    // 2. Config file
    if let Ok(cfg) = config::Config::load() {
        if let Some(year) = cfg.year_override {
            return year;
        }
    }

    // 3. Current year
    template::default_year()
}

/// Resolve license template text using 3-tier chain:
/// 1. Built-in templates (instant, embedded in binary)
/// 2. Disk cache (fast, local)
/// 3. SPDX API (slowest, network)
fn resolve_template(spdx_id: &str) -> anyhow::Result<(String, String)> {
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
    let cache_dir = cache.dir().to_path_buf();
    let registry = registry::Registry::new(&cache_dir)?;
    match registry.fetch_detail(spdx_id) {
        Ok(detail) => {
            // Cache for next time
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

fn cmd_add(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    yes: bool,
) -> anyhow::Result<()> {
    let info = license::get_license_info(spdx)?;
    let author = resolve_author(author)?;
    let year = resolve_year(year);

    if !yes {
        println!("About to add license: {} ({})", info.name, info.id);
        println!("  Author: {}", author);
        println!("  Year:   {}", year);
        println!();
        print!("Continue? [Y/n] ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Resolve template text
    let (raw_text, source) = resolve_template(&info.id)?;
    let content = template::render(&raw_text, &year, &author)?;

    // Determine output filename
    let filename = if info.id.to_uppercase() == info.id {
        format!("LICENSE-{}", info.id)
    } else {
        "LICENSE".to_string()
    };

    // Check for existing file
    if std::path::Path::new(&filename).exists() && !yes {
        println!("{} exists. Overwrite? [y/N] ", filename);
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    std::fs::write(&filename, &content)?;
    println!("✅ Added {} ({}) [from {}]", info.name, info.id, source);

    // Update project manifests
    match project::update_manifest(&info.id, &author, &year) {
        Ok(files) if !files.is_empty() => {
            println!("   Updated: {}", files.join(", "));
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("   Warning: could not update project manifests: {}", e);
        }
    }

    Ok(())
}

fn cmd_list(osi_only: bool, fsf_only: bool, limit: Option<usize>) -> anyhow::Result<()> {
    let index = spdx::SpdxIndex::load()?;
    let mut licenses: Vec<_> = index.licenses.iter().collect();
    licenses.sort_by(|a, b| a.license_id.cmp(&b.license_id));
    let mut count = 0usize;

    for license in &licenses {
        if osi_only && !license.is_osi_approved {
            continue;
        }
        if fsf_only && !license.is_fsf_libre {
            continue;
        }

        let mut flags = Vec::new();
        if license.is_osi_approved {
            flags.push("OSI");
        }
        if license.is_fsf_libre {
            flags.push("FSF");
        }
        if license.is_deprecated_license_id {
            flags.push("DEPRECATED");
        }
        let flag_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(", "))
        };

        let padded = format!("{:<40}", license.license_id);
        println!("{} {}{}", padded, license.name, flag_str);
        count += 1;

        if let Some(max) = limit {
            if count >= max {
                println!("\n... showing {} of {} total", count, index.licenses.len());
                break;
            }
        }
    }

    println!("\n{} licenses found", count);
    Ok(())
}

fn cmd_search(query: &str, osi_only: bool, fsf_only: bool) -> anyhow::Result<()> {
    let index = spdx::SpdxIndex::load()?;
    let results = index.search(query);

    let mut results: Vec<_> = results
        .into_iter()
        .filter(|l| {
            if osi_only && !l.is_osi_approved {
                return false;
            }
            if fsf_only && !l.is_fsf_libre {
                return false;
            }
            true
        })
        .collect();
    results.sort_by(|a, b| a.license_id.cmp(&b.license_id));

    if results.is_empty() {
        eprintln!("No licenses found matching '{}'", query);
        std::process::exit(1);
    }

    for license in &results {
        let mut flags = Vec::new();
        if license.is_osi_approved {
            flags.push("OSI");
        }
        if license.is_fsf_libre {
            flags.push("FSF");
        }
        if license.is_deprecated_license_id {
            flags.push("DEPRECATED");
        }
        let flag_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(", "))
        };

        let padded = format!("{:<40}", license.license_id);
        println!("{} {}{}", padded, license.name, flag_str);
    }

    println!("\n{} licenses found", results.len());
    Ok(())
}

fn cmd_detect() -> anyhow::Result<()> {
    let candidates = [
        "LICENSE",
        "LICENSE.txt",
        "LICENSE.md",
        "COPYING",
        "COPYING.txt",
    ];

    for name in &candidates {
        if std::path::Path::new(name).exists() {
            let content = std::fs::read_to_string(name)?;
            let lower = content.to_lowercase();

            if lower.contains("mit license")
                || lower.contains("permission is hereby granted, free of charge")
            {
                println!("Detected: MIT ({})", name);
                return Ok(());
            }
            if lower.contains("apache license") || lower.contains("version 2.0") {
                println!("Detected: Apache-2.0 ({})", name);
                return Ok(());
            }
            if lower.contains("gnu general public license") && lower.contains("version 3") {
                println!("Detected: GPL-3.0-only ({})", name);
                return Ok(());
            }
            if lower.contains("gnu general public license") && lower.contains("version 2") {
                println!("Detected: GPL-2.0-only ({})", name);
                return Ok(());
            }
            if lower.contains("gnu lesser general public license") {
                println!("Detected: LGPL (variant) ({})", name);
                return Ok(());
            }
            if lower.contains("mozilla public license") && lower.contains("version 2.0") {
                println!("Detected: MPL-2.0 ({})", name);
                return Ok(());
            }
            if lower.contains("bsd") && lower.contains("redistribution and use") {
                if lower.contains("neither the name of") {
                    println!("Detected: BSD-3-Clause ({})", name);
                } else {
                    println!("Detected: BSD-2-Clause ({})", name);
                }
                return Ok(());
            }
            if lower.contains("isc license")
                || lower
                    .contains("permission to use, copy, modify, and/or distribute this software")
            {
                println!("Detected: ISC ({})", name);
                return Ok(());
            }
            if lower.contains("unlicense")
                || lower.contains(
                    "this is free and unencumbered software released into the public domain",
                )
            {
                println!("Detected: Unlicense ({})", name);
                return Ok(());
            }

            println!("Found {} but could not determine license type", name);
            println!("Hint: use 'licencify search' to find the right SPDX ID");
            return Ok(());
        }
    }

    eprintln!("No license file found in current directory");
    eprintln!("Hint: use 'licencify add <SPDX-ID>' to add one");
    std::process::exit(1);
}

fn cmd_update(spdx: &str, author: Option<String>, year: Option<String>) -> anyhow::Result<()> {
    let info = license::get_license_info(spdx)?;
    let author = resolve_author(author)?;
    let year = resolve_year(year);

    // Resolve template text
    let (raw_text, source) = resolve_template(&info.id)?;
    let content = template::render(&raw_text, &year, &author)?;

    // Determine output filename
    let filename = if info.id.to_uppercase() == info.id {
        format!("LICENSE-{}", info.id)
    } else {
        "LICENSE".to_string()
    };

    std::fs::write(&filename, &content)?;
    println!("✅ Updated {} ({}) [from {}]", info.name, info.id, source);

    // Update project manifests
    match project::update_manifest(&info.id, &author, &year) {
        Ok(files) if !files.is_empty() => {
            println!("   Updated: {}", files.join(", "));
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("   Warning: could not update project manifests: {}", e);
        }
    }

    Ok(())
}

fn cmd_cache(action: CacheAction) -> anyhow::Result<()> {
    let cache = cache::LicenseCache::new()?;

    match action {
        CacheAction::Clear => {
            let count = cache.clear()?;
            println!(
                "Cleared {} cached templates from {}",
                count,
                cache.dir().display()
            );
        }
        CacheAction::Info => {
            println!("Cache directory: {}", cache.dir().display());
            println!("Cached templates: {}", cache.count());
        }
    }
    Ok(())
}
