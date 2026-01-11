mod adapters;
mod cache;
mod cli;
mod commands;
mod config;
mod error;
mod git;
mod repository;
mod template;

use clap::Parser;
use cli::{CacheCommands, Cli, Commands, RepoCommands};
use error::Result;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
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
            commands::init_template(path, from_existing, interactive, force)?;
        }

        Commands::Repo(repo_cmd) => match repo_cmd {
            RepoCommands::Add {
                name,
                url,
                default,
                description,
            } => {
                println!("Adding repository '{}' from {}", name, url);
                let mut config = config::Config::load()?;
                let repo = config::Repository {
                    name: name.clone(),
                    url,
                    default,
                    cached_at: None,
                    description,
                };
                config.add_repository(repo)?;
                println!("✓ Repository '{}' added successfully", name);
            }

            RepoCommands::List => {
                let config = config::Config::load()?;
                if config.repositories.is_empty() {
                    println!("No repositories registered.");
                } else {
                    println!("Registered repositories:");
                    for repo in &config.repositories {
                        let default_flag = if repo.default { " [default]" } else { "" };
                        println!("  - {} ({}){}", repo.name, repo.url, default_flag);
                        if let Some(desc) = &repo.description {
                            println!("    {}", desc);
                        }
                    }
                }
            }

            RepoCommands::Remove { name } => {
                let mut config = config::Config::load()?;
                config.remove_repository(&name)?;
                println!("✓ Repository '{}' removed successfully", name);
            }

            RepoCommands::SetDefault { name, value } => {
                let mut config = config::Config::load()?;
                config.set_default(&name, value)?;
                let status = if value { "set as default" } else { "unset as default" };
                println!("✓ Repository '{}' {}", name, status);
            }
        },

        Commands::Pull {
            repositories,
            tools,
            dry_run,
            force,
        } => {
            if repositories.is_empty() {
                eprintln!("Error: No repository specified.");
                eprintln!("Usage: aidot pull <repository>");
                std::process::exit(1);
            }

            // For now, only support single repository (first one)
            let repo_source = repositories[0].clone();
            commands::pull_template(repo_source, tools, dry_run, force)?;
        }

        Commands::Detect => {
            commands::detect_tools()?;
        }

        Commands::Status => {
            println!("Status command (not yet implemented)");
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
            println!("Diff command (not yet implemented)");
            println!("  Repository: {}", repository);
        }
    }

    Ok(())
}
