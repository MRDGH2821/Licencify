mod author;
pub mod cli;
mod commands;
mod config;
mod detect;
pub mod fs;
mod licence_name;
mod licences;
mod process;
mod project;
mod provider;
mod readme;
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
            company,
            email,
            year,
            format,
            yes,
            update_readme,
        } => {
            let do_update = update_readme
                || crate::config::Config::load_effective(None)
                    .ok()
                    .and_then(|c| c.default.update_readme)
                    .unwrap_or(false);
            commands::cmd_add(&spdx, author, company, email, year, format, yes, do_update)
        }
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
            company,
            email,
            year,
            format,
            update_readme,
        } => {
            let do_update = update_readme
                || crate::config::Config::load_effective(None)
                    .ok()
                    .and_then(|c| c.default.update_readme)
                    .unwrap_or(false);
            commands::cmd_update(&spdx, author, company, email, year, format, do_update)
        }
        Commands::Cache { action } => commands::cmd_cache(action),
        Commands::Config { action } => commands::cmd_config(action),
        Commands::Schema { output } => commands::cmd_schema(&output),
    }
}
