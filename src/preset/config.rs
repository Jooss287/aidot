use crate::error::{AidotError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Preset configuration from .aidot-config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetConfig {
    pub metadata: Metadata,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<RulesSection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<DirectorySection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<DirectorySection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<DirectorySection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<DirectorySection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<DirectorySection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<DirectorySection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<DirectorySection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<DirectorySection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesSection {
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorySection {
    pub directory: String,
}

impl PresetConfig {
    /// Load preset configuration from .aidot-config.toml
    pub fn load(path: &Path) -> Result<Self> {
        let config_file = path.join(".aidot-config.toml");

        if !config_file.exists() {
            return Err(AidotError::InvalidPreset(format!(
                "Missing .aidot-config.toml in {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(&config_file)?;
        let config: PresetConfig = toml::from_str(&content)?;
        Ok(config)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_preset_config_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
[metadata]
name = "test-preset"
version = "1.0.0"

[rules]
directory = "rules/"
"#;
        fs::write(temp_dir.path().join(".aidot-config.toml"), config_content).unwrap();

        let loaded = PresetConfig::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.metadata.name, "test-preset");
        assert_eq!(loaded.metadata.version, "1.0.0");
        assert!(loaded.rules.is_some());
    }

    #[test]
    fn test_preset_config_load_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = PresetConfig::load(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_rules_section() {
        let rules = RulesSection {
            files: vec!["rules/test.md".to_string()],
            directory: Some("rules/".to_string()),
        };

        let toml = toml::to_string(&rules).unwrap();
        let deserialized: RulesSection = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.files.len(), 1);
        assert_eq!(deserialized.directory, Some("rules/".to_string()));
    }

    #[test]
    fn test_directory_section() {
        let section = DirectorySection {
            directory: "commands/".to_string(),
        };

        let toml = toml::to_string(&section).unwrap();
        let deserialized: DirectorySection = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.directory, "commands/");
    }

    #[test]
    fn test_metadata() {
        let metadata = Metadata {
            name: "test".to_string(),
            version: "2.0.0".to_string(),
            description: Some("Test description".to_string()),
        };

        let toml = toml::to_string(&metadata).unwrap();
        let deserialized: Metadata = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.version, "2.0.0");
        assert_eq!(
            deserialized.description,
            Some("Test description".to_string())
        );
    }
}
