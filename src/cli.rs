use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

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

#[derive(Clone, ValueEnum)]
pub enum LicenseFormat {
    /// Plain text (licenseText)
    Txt,
    /// HTML (licenseTextHtml)
    Html,
}

impl fmt::Display for LicenseFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LicenseFormat::Txt => write!(f, "txt"),
            LicenseFormat::Html => write!(f, "html"),
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a license to the current project
    Add {
        /// SPDX license identifier (e.g., MIT, Apache-2.0, proprietary)
        spdx: String,

        /// Copyright holder name (default: git config user.name)
        #[arg(short, long)]
        author: Option<String>,

        /// Company name (defaults to author)
        #[arg(long)]
        company: Option<String>,

        /// Contact email address
        #[arg(long)]
        email: Option<String>,

        /// Copyright year (default: current year)
        #[arg(short, long)]
        year: Option<String>,

        /// Output format: txt (default) or html
        #[arg(short, long, default_value = "txt")]
        format: LicenseFormat,

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

        /// Company name (defaults to author)
        #[arg(long)]
        company: Option<String>,

        /// Contact email address
        #[arg(long)]
        email: Option<String>,

        /// Copyright year
        #[arg(short, long)]
        year: Option<String>,

        /// Output format: txt (default) or html
        #[arg(short, long, default_value = "txt")]
        format: LicenseFormat,
    },

    /// Manage local template cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Generate JSON schema for config file
    Schema {
        /// Write schema to file instead of stdout
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Clear all cached templates
    Clear,

    /// Show cache directory location and size
    Info,

    /// Pre-fetch and cache all license templates from SPDX
    FetchAll,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Create default config file
    Init,

    /// Show current configuration
    Show,
}
