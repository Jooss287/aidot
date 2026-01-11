use crate::cache;
use crate::config::Config;
use crate::error::Result;

/// Update cached repositories
pub fn update_cache(repo_name: Option<String>, all: bool) -> Result<()> {
    if all {
        // Update all cached repositories
        let caches = cache::list_caches()?;

        if caches.is_empty() {
            println!("No cached repositories found.");
            return Ok(());
        }

        println!("Updating {} cached repositories...\n", caches.len());

        for cache_name in caches {
            println!("Updating '{}'...", cache_name);
            if let Err(e) = cache::update_cache(&cache_name) {
                eprintln!("  ✗ Failed: {}", e);
            }
            println!();
        }

        println!("✓ All caches updated");
    } else if let Some(name) = repo_name {
        // Update specific repository
        cache::update_cache(&name)?;
        println!("✓ Cache '{}' updated successfully", name);
    } else {
        eprintln!("Error: Specify a repository name or use --all");
        std::process::exit(1);
    }

    Ok(())
}

/// Clear all cached repositories
pub fn clear_cache() -> Result<()> {
    cache::clear_all_caches()
}

/// List all cached repositories
pub fn list_cache() -> Result<()> {
    let caches = cache::list_caches()?;

    if caches.is_empty() {
        println!("No cached repositories.");
        return Ok(());
    }

    println!("Cached repositories:");
    for cache_name in caches {
        let cache_path = cache::get_cache_path(&cache_name)?;
        println!("  - {} ({})", cache_name, cache_path.display());
    }

    Ok(())
}
