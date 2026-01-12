use crate::config::Config;
use crate::error::Result;
use crate::git;
use std::path::PathBuf;

/// Get the cache path for a repository
pub fn get_cache_path(repo_name: &str) -> Result<PathBuf> {
    let cache_dir = Config::cache_dir()?;
    Ok(cache_dir.join(repo_name))
}

/// Ensure a repository is cached locally
/// Returns the path to the cached repository
pub fn ensure_cached(repo_name: &str, repo_url: &str) -> Result<PathBuf> {
    let cache_path = get_cache_path(repo_name)?;

    if cache_path.exists() && git::is_git_repository(&cache_path) {
        // Repository already cached, pull latest changes
        git::pull_repository(&cache_path)?;
    } else {
        // Clone repository
        if cache_path.exists() {
            std::fs::remove_dir_all(&cache_path)?;
        }
        git::clone_repository(repo_url, &cache_path)?;
    }

    Ok(cache_path)
}

/// Update a cached repository
pub fn update_cache(repo_name: &str) -> Result<()> {
    let cache_path = get_cache_path(repo_name)?;

    if !cache_path.exists() {
        return Err(crate::error::AidotError::RepositoryNotFound(format!(
            "Cache not found for repository '{}'",
            repo_name
        )));
    }

    git::pull_repository(&cache_path)?;
    Ok(())
}

/// Clear all cached repositories
pub fn clear_all_caches() -> Result<()> {
    let cache_dir = Config::cache_dir()?;

    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir)?;
        std::fs::create_dir_all(&cache_dir)?;
        println!("✓ All caches cleared");
    } else {
        println!("No caches to clear");
    }

    Ok(())
}

/// List all cached repositories
pub fn list_caches() -> Result<Vec<String>> {
    let cache_dir = Config::cache_dir()?;
    let mut caches = Vec::new();

    if cache_dir.exists() {
        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    caches.push(name.to_string());
                }
            }
        }
    }

    Ok(caches)
}
