mod cli;
mod license;
mod registry;
mod spdx;

use clap::Parser;
use cli::{CacheAction, Cli, Commands};
use spdx::SpdxIndex;

fn main() -> anyhow::Result<()> {
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
        Commands::Update {
            spdx,
            author,
            year,
        } => cmd_update(&spdx, author, year),
        Commands::Cache { action } => cmd_cache(action),
    }
}

fn cmd_add(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    yes: bool,
) -> anyhow::Result<()> {
    let info = license::get_license_info(spdx)?;
    let author = author.unwrap_or_else(|| {
        std::process::Command::new("git")
            .args(["config", "user.name"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "YOUR_NAME".to_string())
    });
    let year = year.unwrap_or_else(|| {
        chrono::Local::now().format("%Y").to_string()
    });

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

    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("licencify")
        .join("templates");
    let registry = registry::Registry::new(&cache_dir)?;
    let detail = registry.fetch_detail(&info.id)?;

    let content = detail
        .license_text
        .replace("<year>", &year)
        .replace("<copyright holders>", &author);

    let filename = if info.id.to_uppercase() == info.id {
        format!("LICENSE-{}", info.id)
    } else {
        "LICENSE".to_string()
    };

    if std::path::Path::new(&filename).exists() && !yes {
        println!("{} already exists. Overwrite? [y/N] ", filename);
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
    println!("✅ Added {} ({}) as {}", info.name, info.id, filename);
    Ok(())
}

fn cmd_list(osi_only: bool, fsf_only: bool, limit: Option<usize>) -> anyhow::Result<()> {
    let index = SpdxIndex::load()?;
    let mut count = 0usize;

    for license in &index.licenses {
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
                println!(
                    "\n... and more (showing {} of {} total)",
                    count,
                    index.licenses.len()
                );
                break;
            }
        }
    }

    println!("\n{} licenses found", count);
    Ok(())
}

fn cmd_search(query: &str, osi_only: bool, fsf_only: bool) -> anyhow::Result<()> {
    let index = SpdxIndex::load()?;
    let results = index.search(query);

    let results: Vec<_> = results
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
    let candidates = ["LICENSE", "LICENSE.txt", "LICENSE.md", "COPYING", "COPYING.txt"];

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
            if lower.contains("apache license") && lower.contains("version 2.0") {
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
                || lower.contains(
                    "permission to use, copy, modify, and/or distribute this software",
                )
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

fn cmd_update(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
) -> anyhow::Result<()> {
    let info = license::get_license_info(spdx)?;
    let author = author.unwrap_or_else(|| {
        std::process::Command::new("git")
            .args(["config", "user.name"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "YOUR_NAME".to_string())
    });
    let year = year.unwrap_or_else(|| {
        chrono::Local::now().format("%Y").to_string()
    });

    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("licencify")
        .join("templates");
    let registry = registry::Registry::new(&cache_dir)?;
    let detail = registry.fetch_detail(&info.id)?;

    let content = detail
        .license_text
        .replace("<year>", &year)
        .replace("<copyright holders>", &author);

    let filename = if info.id.to_uppercase() == info.id {
        format!("LICENSE-{}", info.id)
    } else {
        "LICENSE".to_string()
    };

    std::fs::write(&filename, &content)?;
    println!(
        "✅ Updated to {} ({}) in {}",
        info.name, info.id, filename
    );
    Ok(())
}

fn cmd_cache(action: CacheAction) -> anyhow::Result<()> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("licencify")
        .join("templates");

    match action {
        CacheAction::Clear => {
            if cache_dir.exists() {
                let count = std::fs::read_dir(&cache_dir)?
                    .filter_map(|e| e.ok())
                    .count();
                std::fs::remove_dir_all(&cache_dir)?;
                println!(
                    "Cleared {} cached templates from {}",
                    count,
                    cache_dir.display()
                );
            } else {
                println!(
                    "Cache directory doesn't exist: {}",
                    cache_dir.display()
                );
            }
            Ok(())
        }
        CacheAction::Info => {
            println!("Cache directory: {}", cache_dir.display());
            if cache_dir.exists() {
                let count = std::fs::read_dir(&cache_dir)?
                    .filter_map(|e| e.ok())
                    .count();
                println!("Cached templates: {}", count);
            } else {
                println!("Cache directory doesn't exist yet");
            }
            Ok(())
        }
    }
}
