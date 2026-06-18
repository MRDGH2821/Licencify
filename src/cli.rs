use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "licencify",
    about = "Add open-source licenses to projects",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a license to the current project
    Add {
        /// SPDX license identifier (e.g., MIT, Apache-2.0, GPL-3.0-only)
        spdx: String,

        /// Copyright holder name (default: git config user.name)
        #[arg(short, long)]
        author: Option<String>,

        /// Copyright year (default: current year)
        #[arg(short, long)]
        year: Option<String>,

        /// Skip all prompts and use defaults
        #[arg(short = 'Y', long)]
        yes: bool,
    },

    /// List available licenses
    List {
        /// Show only OSI-approved licenses
        #[arg(long)]
        osi_only: bool,

        /// Show only FSF Libre licenses
        #[arg(long)]
        fsf_only: bool,

        /// Paginate results (max licenses to show)
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Search available licenses by name or ID
    Search {
        /// Search query (matches name or license ID)
        query: String,

        /// Show only OSI-approved licenses
        #[arg(long)]
        osi_only: bool,

        /// Show only FSF Libre licenses
        #[arg(long)]
        fsf_only: bool,
    },

    /// Detect the current project's license
    Detect,

    /// Change the project's license
    Update {
        /// SPDX license identifier to change to
        spdx: String,

        /// Copyright holder name
        #[arg(short, long)]
        author: Option<String>,

        /// Copyright year
        #[arg(short, long)]
        year: Option<String>,
    },

    /// Manage local template cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Clear all cached templates
    Clear,

    /// Show cache directory location and size
    Info,
}
