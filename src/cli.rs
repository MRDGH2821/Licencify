use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

#[derive(Parser)]
#[command(
    name = "licencify",
    about = "Add open-source licenses to projects",
    version
)]
pub struct Cli {
    /// Report each resolved setting's source without printing its value
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Where add and update record the selected licence, author, and format
    #[arg(long, global = true, value_enum)]
    pub config_target: Option<ConfigTargetChoice>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Root entry that should receive a saved licence selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ConfigTargetChoice {
    /// Shared project defaults
    Shared,
    /// Exact-path subdirectory entry
    Subdir,
}

#[derive(Clone, ValueEnum)]
pub enum LicenseFormat {
    /// Plain text (licenseText)
    Txt,
    /// HTML (licenseTextHtml)
    Html,
    /// Markdown (converted from licenseTextHtml)
    Md,
}

impl fmt::Display for LicenseFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LicenseFormat::Txt => write!(f, "txt"),
            LicenseFormat::Html => write!(f, "html"),
            LicenseFormat::Md => write!(f, "md"),
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a license to the current project
    Add {
        /// SPDX license identifier. Omit to use the configured licence.
        spdx: Option<String>,

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

        /// Output format: txt (default), html, or md
        #[arg(short, long, default_value = "txt")]
        format: LicenseFormat,

        /// Skip all prompts and use defaults
        #[arg(short = 'Y', long)]
        yes: bool,

        /// Promote one matching additional licence to primary
        #[arg(long)]
        permit_promotion: bool,

        /// Update README with license badge (if README exists)
        #[arg(long)]
        update_readme: bool,

        /// Skip writing every licence file, including additional licences
        #[arg(long)]
        no_file: bool,
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

    /// Report licence files and README claims without changing files
    Scan {
        /// Expected primary licence ID; does not replace additional licences
        id: Option<String>,
    },

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

        /// Output format: txt (default), html, or md
        #[arg(short, long, default_value = "txt")]
        format: LicenseFormat,

        /// Skip confirmation prompts
        #[arg(short = 'Y', long)]
        yes: bool,

        /// Promote one matching additional licence to primary
        #[arg(long)]
        permit_promotion: bool,

        /// Update README with license badge (if README exists)
        #[arg(long)]
        update_readme: bool,

        /// Skip writing every licence file, including additional licences
        #[arg(long)]
        no_file: bool,
    },

    /// Manage the global SPDX detail cache
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
        /// Output file path (default: licencify-schema.json)
        #[arg(short, long, default_value = "licencify-schema.json")]
        output: String,
    },
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Clear all cached SPDX details
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
