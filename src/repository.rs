use crate::cache;
use crate::config::{Config, SourceType};
use crate::error::Result;
use std::path::PathBuf;

/// Determine if a string is a Git URL
pub fn is_git_url(source: &str) -> bool {
    source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("git@")
        || source.starts_with("ssh://")
}

/// Resolve a repository source to a local path
///
/// # Arguments
/// * `source` - Can be:
///   - A registered repository name (e.g., "common")
///   - A Git URL (e.g., "https://github.com/user/repo")
///   - A local file path (e.g., "./my-preset")
///
/// # Returns
/// The local path to the preset directory
pub fn resolve_repository_source(source: &str) -> Result<PathBuf> {
    // Check if it's a local path (direct input)
    let local_path = PathBuf::from(source);
    if local_path.exists() {
        return Ok(local_path);
    }

    // Check if it's a registered repository name
    let config = Config::load()?;
    if let Some(repo) = config.repositories.iter().find(|r| r.name == source) {
        match repo.source_type {
            SourceType::Local => {
                // Local preset: return path directly (no caching)
                let path = PathBuf::from(&repo.url);
                if path.exists() {
                    println!("Using local preset: {}", repo.url);
                    return Ok(path);
                } else {
                    return Err(crate::error::AidotError::RepositoryNotFound(format!(
                        "Local preset path does not exist: {}",
                        repo.url
                    )));
                }
            }
            SourceType::Git => {
                // Git repository: use cache
                let cache_path = cache::ensure_cached(&repo.name, &repo.url)?;
                return Ok(cache_path);
            }
        }
    }

    // Check if it's a Git URL
    if is_git_url(source) {
        // Create a temporary name from URL
        let repo_name = url_to_repo_name(source);
        let cache_path = cache::ensure_cached(&repo_name, source)?;
        return Ok(cache_path);
    }

    Err(crate::error::AidotError::RepositoryNotFound(format!(
        "Repository '{}' not found. It must be a local path, registered repository name, or Git URL.",
        source
    )))
}

/// Convert a Git URL to a repository name for caching
fn url_to_repo_name(url: &str) -> String {
    // Extract repo name from URL
    // e.g., https://github.com/user/repo.git -> repo
    url.trim_end_matches(".git")
        .split('/')
        .next_back()
        .unwrap_or("temp-repo")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_url() {
        assert!(is_git_url("https://github.com/user/repo"));
        assert!(is_git_url("http://github.com/user/repo"));
        assert!(is_git_url("git@github.com:user/repo.git"));
        assert!(is_git_url("ssh://git@github.com/user/repo"));
        assert!(!is_git_url("./local/path"));
        assert!(!is_git_url("repo-name"));
    }

    #[test]
    fn test_url_to_repo_name() {
        assert_eq!(url_to_repo_name("https://github.com/user/repo.git"), "repo");
        assert_eq!(url_to_repo_name("https://github.com/user/repo"), "repo");
        assert_eq!(
            url_to_repo_name("git@github.com:user/my-preset.git"),
            "my-preset"
        );
    }
}
