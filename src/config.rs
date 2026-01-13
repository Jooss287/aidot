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

/// Source type for repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    #[default]
    Git,
    Local,
}

/// Repository entry in global configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub source_type: SourceType,
    #[serde(default)]
    pub default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// History entry for tracking applied presets
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
            source_type: SourceType::Git,
            default: true,
            cached_at: Some("2026-01-11T00:00:00Z".to_string()),
            description: Some("Test repository".to_string()),
        };

        let toml = toml::to_string(&repo).unwrap();
        let deserialized: Repository = toml::from_str(&toml).unwrap();

        assert_eq!(repo.name, deserialized.name);
        assert_eq!(repo.url, deserialized.url);
        assert_eq!(repo.default, deserialized.default);
        assert_eq!(repo.source_type, deserialized.source_type);
    }

    #[test]
    fn test_local_repository_serialization() {
        let repo = Repository {
            name: "local-test".to_string(),
            url: "/home/user/presets/my-preset".to_string(),
            source_type: SourceType::Local,
            default: false,
            cached_at: None,
            description: Some("Local preset".to_string()),
        };

        let toml = toml::to_string(&repo).unwrap();
        let deserialized: Repository = toml::from_str(&toml).unwrap();

        assert_eq!(repo.source_type, SourceType::Local);
        assert_eq!(deserialized.source_type, SourceType::Local);
    }

    #[test]
    fn test_source_type_default() {
        let source_type = SourceType::default();
        assert_eq!(source_type, SourceType::Git);
    }

    #[test]
    fn test_history_entry() {
        let entry = HistoryEntry {
            project: "/home/user/project".to_string(),
            timestamp: "2026-01-12T10:00:00Z".to_string(),
            repositories: vec!["common".to_string(), "team-config".to_string()],
        };

        let toml = toml::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.project, "/home/user/project");
        assert_eq!(deserialized.repositories.len(), 2);
    }

    #[test]
    fn test_config_with_repositories() {
        let mut config = Config::default();

        let repo1 = Repository {
            name: "repo1".to_string(),
            url: "https://github.com/test/repo1".to_string(),
            source_type: SourceType::Git,
            default: true,
            cached_at: None,
            description: None,
        };

        let repo2 = Repository {
            name: "repo2".to_string(),
            url: "/local/path".to_string(),
            source_type: SourceType::Local,
            default: false,
            cached_at: None,
            description: Some("Local repo".to_string()),
        };

        config.repositories.push(repo1);
        config.repositories.push(repo2);

        let toml = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.repositories.len(), 2);
        assert_eq!(deserialized.repositories[0].name, "repo1");
        assert_eq!(deserialized.repositories[1].source_type, SourceType::Local);
    }

    #[test]
    fn test_repository_without_optional_fields() {
        let toml_str = r#"
            name = "minimal"
            url = "https://example.com/repo"
        "#;

        let repo: Repository = toml::from_str(toml_str).unwrap();
        assert_eq!(repo.name, "minimal");
        assert_eq!(repo.source_type, SourceType::Git); // default
        assert!(!repo.default); // default is false
        assert!(repo.cached_at.is_none());
        assert!(repo.description.is_none());
    }
}
