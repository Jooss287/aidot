use crate::error::{AidotError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Global configuration stored in ~/.aidot/config.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub repositories: Vec<Repository>,

    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

/// Repository entry in global configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// History entry for tracking applied templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub project: String,
    pub timestamp: String,
    pub repositories: Vec<String>,
}

impl Config {
    /// Get the global config directory path (~/.aidot/)
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| AidotError::ConfigParse("Could not find home directory".to_string()))?;
        Ok(home.join(".aidot"))
    }

    /// Get the global config file path (~/.aidot/config.toml)
    pub fn config_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Get the cache directory path (~/.aidot/cache/)
    pub fn cache_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("cache"))
    }

    /// Load configuration from ~/.aidot/config.toml
    /// Creates a default config if it doesn't exist
    pub fn load() -> Result<Self> {
        let config_file = Self::config_file()?;

        if !config_file.exists() {
            // Create default config
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_file)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to ~/.aidot/config.toml
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_file = Self::config_file()?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        // Create cache directory if it doesn't exist
        let cache_dir = Self::cache_dir()?;
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_file, content)?;
        Ok(())
    }

    /// Add a repository to the configuration
    pub fn add_repository(&mut self, repo: Repository) -> Result<()> {
        // Check if repository with same name already exists
        if self.repositories.iter().any(|r| r.name == repo.name) {
            return Err(AidotError::ConfigParse(format!(
                "Repository '{}' already exists",
                repo.name
            )));
        }
        self.repositories.push(repo);
        self.save()
    }

    /// Remove a repository by name
    pub fn remove_repository(&mut self, name: &str) -> Result<()> {
        let initial_len = self.repositories.len();
        self.repositories.retain(|r| r.name != name);

        if self.repositories.len() == initial_len {
            return Err(AidotError::RepositoryNotFound(name.to_string()));
        }

        self.save()
    }

    /// Get a repository by name
    #[allow(dead_code)]
    pub fn get_repository(&self, name: &str) -> Option<&Repository> {
        self.repositories.iter().find(|r| r.name == name)
    }

    /// Get all default repositories
    #[allow(dead_code)]
    pub fn get_default_repositories(&self) -> Vec<&Repository> {
        self.repositories.iter().filter(|r| r.default).collect()
    }

    /// Set default flag for a repository
    pub fn set_default(&mut self, name: &str, default: bool) -> Result<()> {
        let repo = self.repositories.iter_mut()
            .find(|r| r.name == name)
            .ok_or_else(|| AidotError::RepositoryNotFound(name.to_string()))?;

        repo.default = default;
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.repositories.len(), 0);
        assert_eq!(config.history.len(), 0);
    }

    #[test]
    fn test_repository_serialization() {
        let repo = Repository {
            name: "test".to_string(),
            url: "https://github.com/test/repo".to_string(),
            default: true,
            cached_at: Some("2026-01-11T00:00:00Z".to_string()),
            description: Some("Test repository".to_string()),
        };

        let toml = toml::to_string(&repo).unwrap();
        let deserialized: Repository = toml::from_str(&toml).unwrap();

        assert_eq!(repo.name, deserialized.name);
        assert_eq!(repo.url, deserialized.url);
        assert_eq!(repo.default, deserialized.default);
    }
}
