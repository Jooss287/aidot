mod cli;
mod commands;
mod config;
mod error;
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
            println!("Pull command (not yet implemented)");
            println!("  Repositories: {:?}", repositories);
            println!("  Tools: {:?}", tools);
            println!("  Dry run: {}", dry_run);
            println!("  Force: {}", force);
        }

        Commands::Detect => {
            println!("Detect command (not yet implemented)");
        }

        Commands::Status => {
            println!("Status command (not yet implemented)");
        }

        Commands::Cache(cache_cmd) => match cache_cmd {
            CacheCommands::Update { name, all } => {
                println!("Cache update command (not yet implemented)");
                println!("  Name: {:?}", name);
                println!("  All: {}", all);
            }
            CacheCommands::Clear => {
                println!("Cache clear command (not yet implemented)");
            }
        },

        Commands::Diff { repository } => {
            println!("Diff command (not yet implemented)");
            println!("  Repository: {}", repository);
        }
    }

    Ok(())
}
