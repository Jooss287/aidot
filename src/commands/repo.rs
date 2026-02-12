use crate::config::{self, Config, Repository, SourceType};
use crate::error::{AidotError, Result};
use crate::git;
use crate::repository;
use colored::Colorize;
use std::path::PathBuf;

/// Process a local path and validate it as a preset directory
fn process_local_path(url: &str) -> Result<(String, SourceType)> {
    let path = PathBuf::from(url);
    let absolute_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(&path)
    };

    // Canonicalize to resolve . and ..
    let canonical = std::fs::canonicalize(&absolute_path).map_err(|_| {
        AidotError::RepositoryNotFound(format!(
            "Local path does not exist: {}",
            absolute_path.display()
        ))
    })?;

    // Verify it's a directory
    if !canonical.is_dir() {
        return Err(AidotError::RepositoryNotFound(format!(
            "Path is not a directory: {}",
            canonical.display()
        )));
    }

    // Verify .aidot-config.toml exists
    let config_file = canonical.join(".aidot-config.toml");
    if !config_file.exists() {
        return Err(AidotError::InvalidPreset(format!(
            "Not a valid preset directory (missing .aidot-config.toml): {}",
            canonical.display()
        )));
    }

    Ok((canonical.to_string_lossy().to_string(), SourceType::Local))
}

/// Add a new repository or local preset
pub fn add_repo(
    name: String,
    url: String,
    local: bool,
    default: bool,
    description: Option<String>,
) -> Result<()> {
    // Determine source type: explicit --local flag, URL pattern, or auto-detect local path
    let (resolved_url, source_type, is_local) = if local {
        // Explicit --local flag: treat as local path
        let (resolved, source) = process_local_path(&url)?;
        (resolved, source, true)
    } else if repository::is_git_url(&url) {
        // URL pattern detected: treat as Git repository
        git::check_git_available()?;
        (url.clone(), config::SourceType::Git, false)
    } else {
        // Not a URL pattern: check if it's an existing local path
        let path = PathBuf::from(&url);
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
            return Err(AidotError::InvalidInput(format!(
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

    let mut cfg = Config::load()?;
    let repo = Repository {
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

    Ok(())
}

/// List all registered repositories
pub fn list_repos() -> Result<()> {
    let cfg = Config::load()?;
    if cfg.repositories.is_empty() {
        println!("{}", "No repositories registered.".yellow());
        println!(
            "{}",
            "Use 'aidot repo add <name> <url>' to register a preset repository.".dimmed()
        );
    } else {
        println!("{}", "Registered repositories:".cyan().bold());
        for repo in &cfg.repositories {
            let mut flags = Vec::new();
            if repo.source_type == SourceType::Local {
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

    Ok(())
}

/// Remove a registered repository
pub fn remove_repo(name: &str) -> Result<()> {
    let mut config = Config::load()?;
    config.remove_repository(name)?;
    println!(
        "{} Repository '{}' removed successfully",
        "✓".green(),
        name.white().bold()
    );

    Ok(())
}

/// Set or unset a repository as default
pub fn set_default_repo(name: &str, value: bool) -> Result<()> {
    let mut config = Config::load()?;
    config.set_default(name, value)?;
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

    Ok(())
}
