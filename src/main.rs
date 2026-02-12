mod adapters;
mod cache;
mod cli;
mod commands;
mod config;
mod error;
mod git;
mod preset;
mod repository;

use clap::Parser;
use cli::{CacheCommands, Cli, Commands, RepoCommands};
use colored::Colorize;
use error::Result;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            from_existing,
            interactive,
            force,
            path,
        } => {
            commands::init_preset(path, from_existing, interactive, force)?;
        }

        Commands::Repo(repo_cmd) => match repo_cmd {
            RepoCommands::Add {
                name,
                url,
                local,
                default,
                description,
            } => {
                commands::add_repo(name, url, local, default, description)?;
            }

            RepoCommands::List => {
                commands::list_repos()?;
            }

            RepoCommands::Remove { name } => {
                commands::remove_repo(&name)?;
            }

            RepoCommands::SetDefault { name, value } => {
                commands::set_default_repo(&name, value)?;
            }
        },

        Commands::Pull {
            repositories,
            tools,
            dry_run,
            force,
            skip,
        } => {
            let repos_to_apply: Vec<String> = if repositories.is_empty() {
                // Apply all default repositories
                let cfg = config::Config::load()?;
                let defaults: Vec<String> = cfg
                    .repositories
                    .iter()
                    .filter(|r| r.default)
                    .map(|r| r.name.clone())
                    .collect();

                if defaults.is_empty() {
                    println!("{}", "No default repositories configured.".yellow());
                    println!(
                        "{}",
                        "Use 'aidot repo add <name> <url> --default' to register a default repository."
                            .dimmed()
                    );
                    println!(
                        "{}",
                        "Or specify a repository: 'aidot pull <repository>'".dimmed()
                    );
                    return Ok(());
                }

                println!(
                    "{} {}",
                    "Applying".cyan(),
                    format!("{} default repository(s)...", defaults.len()).white()
                );
                defaults
            } else {
                repositories
            };

            // Apply each repository sequentially
            for (i, repo_source) in repos_to_apply.iter().enumerate() {
                if repos_to_apply.len() > 1 {
                    println!(
                        "\n{} [{}/{}] {}",
                        "═══".cyan(),
                        (i + 1).to_string().white().bold(),
                        repos_to_apply.len().to_string().white().bold(),
                        repo_source.white().bold()
                    );
                }
                commands::pull_preset(repo_source.clone(), tools.clone(), dry_run, force, skip)?;
            }

            if repos_to_apply.len() > 1 {
                println!(
                    "\n{} {} repositories applied successfully!",
                    "✓".green().bold(),
                    repos_to_apply.len().to_string().white().bold()
                );
            }
        }

        Commands::Detect => {
            commands::detect_tools()?;
        }

        Commands::Status => {
            commands::show_status()?;
        }

        Commands::Cache(cache_cmd) => match cache_cmd {
            CacheCommands::Update { name, all } => {
                commands::update_cache(name, all)?;
            }
            CacheCommands::Clear => {
                commands::clear_cache()?;
            }
        },

        Commands::Diff { repository } => {
            commands::show_diff(repository)?;
        }

        Commands::Update { check, prerelease } => {
            commands::check_update(check, prerelease)?;
        }
    }

    Ok(())
}
