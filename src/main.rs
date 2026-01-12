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
            commands::init_template(path, from_existing, interactive, force)?;
        }

        Commands::Repo(repo_cmd) => match repo_cmd {
            RepoCommands::Add {
                name,
                url,
                local,
                default,
                description,
            } => {
                let (resolved_url, source_type) = if local {
                    // Convert to absolute path
                    let path = std::path::PathBuf::from(&url);
                    let absolute_path = if path.is_absolute() {
                        path
                    } else {
                        std::env::current_dir()?.join(&path)
                    };

                    // Canonicalize to resolve . and ..
                    let canonical = std::fs::canonicalize(&absolute_path).map_err(|_| {
                        error::AidotError::RepositoryNotFound(format!(
                            "Local path does not exist: {}",
                            absolute_path.display()
                        ))
                    })?;

                    // Verify it's a directory
                    if !canonical.is_dir() {
                        return Err(error::AidotError::RepositoryNotFound(format!(
                            "Path is not a directory: {}",
                            canonical.display()
                        ))
                        .into());
                    }

                    (
                        canonical.to_string_lossy().to_string(),
                        config::SourceType::Local,
                    )
                } else {
                    (url.clone(), config::SourceType::Git)
                };

                let type_label = if local {
                    "local template".yellow()
                } else {
                    "repository".cyan()
                };
                println!(
                    "{} {} '{}' from {}",
                    "Adding".cyan(),
                    type_label,
                    name.white().bold(),
                    resolved_url.dimmed()
                );

                let mut cfg = config::Config::load()?;
                let repo = config::Repository {
                    name: name.clone(),
                    url: resolved_url,
                    source_type,
                    default,
                    cached_at: None,
                    description,
                };
                cfg.add_repository(repo)?;

                let default_msg = if default {
                    format!(" {}", "[default]".green())
                } else {
                    String::new()
                };
                println!(
                    "{} {} '{}' added successfully{}",
                    "✓".green(),
                    if local { "Local template" } else { "Repository" },
                    name.white().bold(),
                    default_msg
                );
            }

            RepoCommands::List => {
                let cfg = config::Config::load()?;
                if cfg.repositories.is_empty() {
                    println!("{}", "No repositories registered.".yellow());
                    println!(
                        "{}",
                        "Use 'aidot repo add <name> <url>' to register a template repository."
                            .dimmed()
                    );
                } else {
                    println!("{}", "Registered repositories:".cyan().bold());
                    for repo in &cfg.repositories {
                        let mut flags = Vec::new();
                        if repo.source_type == config::SourceType::Local {
                            flags.push("local".yellow().to_string());
                        }
                        if repo.default {
                            flags.push("default".green().to_string());
                        }
                        let flags_str = if flags.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", flags.join("] ["))
                        };
                        println!(
                            "  {} {} {}{}",
                            "•".cyan(),
                            repo.name.white().bold(),
                            repo.url.dimmed(),
                            flags_str
                        );
                        if let Some(desc) = &repo.description {
                            println!("    {}", desc.dimmed());
                        }
                    }
                }
            }

            RepoCommands::Remove { name } => {
                let mut config = config::Config::load()?;
                config.remove_repository(&name)?;
                println!(
                    "{} Repository '{}' removed successfully",
                    "✓".green(),
                    name.white().bold()
                );
            }

            RepoCommands::SetDefault { name, value } => {
                let mut config = config::Config::load()?;
                config.set_default(&name, value)?;
                let status = if value {
                    "set as default".green()
                } else {
                    "unset as default".yellow()
                };
                println!(
                    "{} Repository '{}' {}",
                    "✓".green(),
                    name.white().bold(),
                    status
                );
            }
        },

        Commands::Pull {
            repositories,
            tools,
            dry_run,
            force,
        } => {
            if repositories.is_empty() {
                eprintln!("{} {}", "Error:".red().bold(), "No repository specified.");
                eprintln!(
                    "{} {}",
                    "Usage:".dimmed(),
                    "aidot pull <repository>".white()
                );
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
            println!(
                "{} {}",
                "Diff command".yellow(),
                "(not yet implemented)".dimmed()
            );
            println!("  {} {}", "Repository:".dimmed(), repository.white());
        }
    }

    Ok(())
}
