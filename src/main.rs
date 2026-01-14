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
                // Helper function to check if input looks like a URL
                fn is_url(input: &str) -> bool {
                    input.starts_with("http://")
                        || input.starts_with("https://")
                        || input.starts_with("git@")
                        || input.starts_with("ssh://")
                        || input.starts_with("git://")
                }

                // Helper function to process local path
                fn process_local_path(url: &str) -> Result<(String, config::SourceType)> {
                    let path = std::path::PathBuf::from(url);
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
                        )));
                    }

                    // Verify .aidot-config.toml exists
                    let config_file = canonical.join(".aidot-config.toml");
                    if !config_file.exists() {
                        return Err(error::AidotError::InvalidPreset(format!(
                            "Not a valid preset directory (missing .aidot-config.toml): {}",
                            canonical.display()
                        )));
                    }

                    Ok((
                        canonical.to_string_lossy().to_string(),
                        config::SourceType::Local,
                    ))
                }

                // Determine source type: explicit --local flag, URL pattern, or auto-detect local path
                let (resolved_url, source_type, is_local) = if local {
                    // Explicit --local flag: treat as local path
                    let (resolved, source) = process_local_path(&url)?;
                    (resolved, source, true)
                } else if is_url(&url) {
                    // URL pattern detected: treat as Git repository
                    git::check_git_available()?;
                    (url.clone(), config::SourceType::Git, false)
                } else {
                    // Not a URL pattern: check if it's an existing local path
                    let path = std::path::PathBuf::from(&url);
                    let absolute_path = if path.is_absolute() {
                        path
                    } else {
                        std::env::current_dir()?.join(&path)
                    };

                    if absolute_path.exists() {
                        // Path exists: auto-detect as local preset
                        let (resolved, source) = process_local_path(&url)?;
                        println!(
                            "{} Auto-detected as local path. Use --local flag to make this explicit.",
                            "Note:".yellow()
                        );
                        (resolved, source, true)
                    } else {
                        // Path doesn't exist and not a URL pattern: show helpful error
                        return Err(error::AidotError::InvalidInput(format!(
                            "'{}' is neither a valid URL (http://, https://, git@, ssh://, git://) \
                            nor an existing local path.\n\
                            \n\
                            For remote repositories, use a valid Git URL:\n\
                            \x20 aidot repo add {} https://github.com/user/repo.git\n\
                            \n\
                            For local presets, use --local flag with an existing directory:\n\
                            \x20 aidot repo add {} /path/to/preset --local",
                            url, name, name
                        )));
                    }
                };

                let local = is_local;

                let type_label = if local {
                    "local preset".yellow()
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
                    if local { "Local preset" } else { "Repository" },
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
                        "Use 'aidot repo add <name> <url>' to register a preset repository."
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
