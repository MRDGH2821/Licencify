#![allow(dead_code)]
mod author;
pub mod cli;
mod commands;
mod config;
mod detect;
mod licences;
mod project;
mod provider;
mod resolution;
mod spdx;
mod template;

use clap::Parser;
use cli::{Cli, Commands};

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add {
            spdx,
            author,
            year,
            format,
            yes,
        } => commands::cmd_add(&spdx, author, year, format, yes),
        Commands::List {
            osi_only,
            fsf_only,
            limit,
        } => commands::cmd_list(osi_only, fsf_only, limit),
        Commands::Search {
            query,
            osi_only,
            fsf_only,
        } => commands::cmd_search(&query, osi_only, fsf_only),
        Commands::Detect => commands::cmd_detect(),
        Commands::Update {
            spdx,
            author,
            year,
            format,
        } => commands::cmd_update(&spdx, author, year, format),
        Commands::Cache { action } => commands::cmd_cache(action),
        Commands::Config { action } => commands::cmd_config(action),
    }
}
