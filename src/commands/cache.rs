use crate::cache;
use crate::config::{Config, SourceType};
use crate::error::Result;
use colored::Colorize;

/// Update cached repositories
pub fn update_cache(repo_name: Option<String>, all: bool) -> Result<()> {
    let config = Config::load()?;

    if all {
        // Update all cached repositories (skip local presets)
        let caches = cache::list_caches()?;

        if caches.is_empty() {
            println!("{}", "No cached repositories found.".yellow());
            return Ok(());
        }

        println!(
            "{} {} {}\n",
            "Updating".cyan(),
            caches.len().to_string().white().bold(),
            "cached repositories...".cyan()
        );

        let mut success_count = 0;
        let mut skip_count = 0;
        let mut fail_count = 0;

        for cache_name in caches {
            // Check if this is a local preset
            if let Some(repo) = config.repositories.iter().find(|r| r.name == cache_name) {
                if repo.source_type == SourceType::Local {
                    println!(
                        "  {} '{}': {}",
                        "⊘".yellow(),
                        cache_name.white(),
                        "local preset (no caching needed)".dimmed()
                    );
                    skip_count += 1;
                    continue;
                }
            }

            print!("  {} '{}'... ", "↻".cyan(), cache_name.white());
            match cache::update_cache(&cache_name) {
                Ok(_) => {
                    println!("{}", "done".green());
                    success_count += 1;
                }
                Err(e) => {
                    println!("{}", "failed".red());
                    eprintln!("    {} {}", "Error:".red(), e);
                    fail_count += 1;
                }
            }
        }

        println!();
        if fail_count == 0 {
            println!(
                "{} {} updated, {} skipped",
                "✓".green(),
                success_count.to_string().green(),
                skip_count.to_string().yellow()
            );
        } else {
            println!(
                "{} {} updated, {} skipped, {} failed",
                "!".yellow(),
                success_count.to_string().green(),
                skip_count.to_string().yellow(),
                fail_count.to_string().red()
            );
        }
    } else if let Some(name) = repo_name {
        // Check if this is a local preset
        if let Some(repo) = config.repositories.iter().find(|r| r.name == name) {
            if repo.source_type == SourceType::Local {
                println!(
                    "{} '{}' is a local preset {}",
                    "⊘".yellow(),
                    name.white().bold(),
                    "(no caching needed)".dimmed()
                );
                return Ok(());
            }
        }

        // Update specific repository
        println!(
            "{} '{}'...",
            "Updating cache for".cyan(),
            name.white().bold()
        );
        cache::update_cache(&name)?;
        println!(
            "{} Cache '{}' updated successfully",
            "✓".green(),
            name.white().bold()
        );
    } else {
        eprintln!(
            "{} Specify a repository name or use --all",
            "Error:".red().bold()
        );
        eprintln!(
            "{} {}",
            "Usage:".dimmed(),
            "aidot cache update <name> or aidot cache update --all".white()
        );
        std::process::exit(1);
    }

    Ok(())
}

/// Clear all cached repositories
pub fn clear_cache() -> Result<()> {
    println!("{}", "Clearing all cached repositories...".cyan());
    cache::clear_all_caches()?;
    println!("{} {}", "✓".green(), "All caches cleared".green().bold());
    Ok(())
}
