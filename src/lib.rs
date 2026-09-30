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
    let yes = match &cli.command {
        Commands::Add { yes, .. } | Commands::Update { yes, .. } => *yes,
        _ => false,
    };
    config::set_cli_context(config::CliContext {
        verbose: cli.verbose,
        yes,
        config_target: cli.config_target.map(|target| match target {
            cli::ConfigTargetChoice::Shared => config::ConfigWriteTarget::SharedDefaults,
            cli::ConfigTargetChoice::Subdir => config::ConfigWriteTarget::ExactSubdir,
        }),
    });

    match cli.command {
        Commands::Add {
            spdx,
            author,
            company,
            email,
            year,
            format,
            yes,
            permit_promotion,
            update_readme,
        } => commands::cmd_add(
            spdx.as_deref(),
            author,
            company,
            email,
            year,
            format,
            yes,
            permit_promotion,
            update_readme,
        ),
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
        Commands::Scan { id } => commands::cmd_scan(id.as_deref()),
        Commands::Update {
            spdx,
            author,
            company,
            email,
            year,
            format,
            yes,
            permit_promotion,
            update_readme,
        } => commands::cmd_update(
            &spdx,
            author,
            company,
            email,
            year,
            format,
            yes,
            permit_promotion,
            update_readme,
        ),
        Commands::Cache { action } => commands::cmd_cache(action),
        Commands::Config { action } => commands::cmd_config(action),
        Commands::Schema { output } => commands::cmd_schema(&output),
    }
}
