use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "aidot")]
#[command(version, about = "AI dotfiles - Manage LLM tool configurations")]
#[command(long_about = "aidot (AI dotfiles) is a CLI tool that manages LLM tool configurations \
across multiple AI coding assistants. It fetches tool-agnostic configuration presets \
from Git repositories and automatically converts them to the appropriate format for each \
detected LLM tool (Claude Code, Cursor, GitHub Copilot, etc.).")]
#[command(styles = get_styles())]
#[command(after_help = "Examples:
  aidot init                    Initialize a new preset repository
  aidot repo add common <url>   Register a preset repository
  aidot pull common             Apply preset to all detected tools
  aidot pull common --tools cursor,claude  Apply to specific tools only
  aidot detect                  Show installed LLM tools
  aidot status                  Show current configuration status")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress output (quiet mode)
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            clap::builder::styling::AnsiColor::Cyan
                .on_default()
                .bold(),
        )
        .header(
            clap::builder::styling::AnsiColor::Cyan
                .on_default()
                .bold(),
        )
        .literal(clap::builder::styling::AnsiColor::Green.on_default())
        .placeholder(clap::builder::styling::AnsiColor::Yellow.on_default())
        .valid(
            clap::builder::styling::AnsiColor::Green
                .on_default()
                .bold(),
        )
        .invalid(
            clap::builder::styling::AnsiColor::Red
                .on_default()
                .bold(),
        )
        .error(
            clap::builder::styling::AnsiColor::Red
                .on_default()
                .bold(),
        )
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new preset repository
    Init {
        /// Create preset from existing LLM configurations
        #[arg(long)]
        from_existing: bool,

        /// Interactive preset creation
        #[arg(long)]
        interactive: bool,

        /// Force overwrite if preset already exists
        #[arg(short, long)]
        force: bool,

        /// Target directory (default: current directory)
        #[arg(value_name = "DIR")]
        path: Option<String>,
    },

    /// Manage preset repositories
    #[command(subcommand)]
    Repo(RepoCommands),

    /// Pull and apply preset configurations
    Pull {
        /// Repository name or URL (if empty, applies all default repositories)
        #[arg(value_name = "REPO")]
        repositories: Vec<String>,

        /// Apply to specific tools only (comma-separated: cursor,claude,copilot)
        #[arg(long, value_delimiter = ',')]
        tools: Option<Vec<String>>,

        /// Preview changes without applying them
        #[arg(long)]
        dry_run: bool,

        /// Force overwrite existing files without asking
        #[arg(short, long, conflicts_with = "skip")]
        force: bool,

        /// Skip existing files without asking
        #[arg(short, long, conflicts_with = "force")]
        skip: bool,
    },

    /// Detect installed LLM tools
    Detect,

    /// Show current configuration status
    Status,

    /// Manage cache
    #[command(subcommand)]
    Cache(CacheCommands),

    /// Show diff between preset and current config
    Diff {
        /// Repository name, local path, or Git URL
        #[arg(value_name = "REPO")]
        repository: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum RepoCommands {
    /// Add a new preset repository
    Add {
        /// Repository name
        #[arg(value_name = "NAME")]
        name: String,

        /// Repository URL or local path
        #[arg(value_name = "URL_OR_PATH")]
        url: String,

        /// Register as a local preset (path will be converted to absolute path)
        #[arg(long)]
        local: bool,

        /// Set as default repository
        #[arg(long)]
        default: bool,

        /// Repository description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// List registered repositories
    List,

    /// Remove a repository
    Remove {
        /// Repository name
        #[arg(value_name = "NAME")]
        name: String,
    },

    /// Set or unset default flag for a repository
    SetDefault {
        /// Repository name
        #[arg(value_name = "NAME")]
        name: String,

        /// Default flag value (true or false)
        #[arg(value_name = "VALUE")]
        value: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// Update cached repositories
    Update {
        /// Repository name (if empty, updates all)
        #[arg(value_name = "NAME")]
        name: Option<String>,

        /// Update all cached repositories
        #[arg(long)]
        all: bool,
    },

    /// Clear all cached repositories
    Clear,
}
