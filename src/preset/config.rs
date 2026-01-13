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
    #[serde(default = "default_merge_strategy")]
    pub merge_strategy: MergeStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorySection {
    pub directory: String,
    #[serde(default = "default_merge_strategy")]
    pub merge_strategy: MergeStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    #[default]
    Concat,
    Replace,
}

fn default_merge_strategy() -> MergeStrategy {
    MergeStrategy::Concat
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

    /// Save preset configuration to .aidot-config.toml
    pub fn save(&self, path: &Path) -> Result<()> {
        let config_file = path.join(".aidot-config.toml");
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_file, content)?;
        Ok(())
    }

    /// Create a default preset configuration
    pub fn default_preset(name: &str) -> Self {
        PresetConfig {
            metadata: Metadata {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: Some("LLM configuration preset".to_string()),
            },
            rules: Some(RulesSection {
                files: vec!["rules/code-style.md".to_string()],
                merge_strategy: MergeStrategy::Concat,
            }),
            memory: Some(DirectorySection {
                directory: "memory/".to_string(),
                merge_strategy: MergeStrategy::Concat,
            }),
            commands: Some(DirectorySection {
                directory: "commands/".to_string(),
                merge_strategy: MergeStrategy::Replace,
            }),
            mcp: Some(DirectorySection {
                directory: "mcp/".to_string(),
                merge_strategy: MergeStrategy::Concat,
            }),
            hooks: Some(DirectorySection {
                directory: "hooks/".to_string(),
                merge_strategy: MergeStrategy::Replace,
            }),
            agents: Some(DirectorySection {
                directory: "agents/".to_string(),
                merge_strategy: MergeStrategy::Replace,
            }),
            skills: Some(DirectorySection {
                directory: "skills/".to_string(),
                merge_strategy: MergeStrategy::Replace,
            }),
            settings: Some(DirectorySection {
                directory: "settings/".to_string(),
                merge_strategy: MergeStrategy::Concat,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_preset() {
        let config = PresetConfig::default_preset("test-preset");
        assert_eq!(config.metadata.name, "test-preset");
        assert_eq!(config.metadata.version, "1.0.0");
        assert!(config.rules.is_some());
        assert!(config.memory.is_some());
    }

    #[test]
    fn test_merge_strategy_serialization() {
        let concat = MergeStrategy::Concat;
        let replace = MergeStrategy::Replace;

        let concat_str = serde_json::to_string(&concat).unwrap();
        let replace_str = serde_json::to_string(&replace).unwrap();

        assert_eq!(concat_str, "\"concat\"");
        assert_eq!(replace_str, "\"replace\"");
    }

    #[test]
    fn test_merge_strategy_default() {
        let strategy = MergeStrategy::default();
        assert_eq!(strategy, MergeStrategy::Concat);
    }

    #[test]
    fn test_merge_strategy_toml_serialization() {
        // TOML doesn't support bare enums, so test within a struct
        let section = DirectorySection {
            directory: "test/".to_string(),
            merge_strategy: MergeStrategy::Concat,
        };

        let toml_str = toml::to_string(&section).unwrap();
        assert!(toml_str.contains("concat"));

        let section_replace = DirectorySection {
            directory: "test/".to_string(),
            merge_strategy: MergeStrategy::Replace,
        };

        let toml_replace = toml::to_string(&section_replace).unwrap();
        assert!(toml_replace.contains("replace"));

        // Deserialization
        let deserialized: DirectorySection = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.merge_strategy, MergeStrategy::Concat);

        let deserialized_replace: DirectorySection = toml::from_str(&toml_replace).unwrap();
        assert_eq!(deserialized_replace.merge_strategy, MergeStrategy::Replace);
    }

    #[test]
    fn test_preset_config_serialization() {
        let config = PresetConfig::default_preset("test");
        let toml = toml::to_string_pretty(&config).unwrap();
        let deserialized: PresetConfig = toml::from_str(&toml).unwrap();

        assert_eq!(config.metadata.name, deserialized.metadata.name);
    }

    #[test]
    fn test_preset_config_all_sections() {
        let config = PresetConfig::default_preset("full-test");

        assert!(config.rules.is_some());
        assert!(config.memory.is_some());
        assert!(config.commands.is_some());
        assert!(config.mcp.is_some());
        assert!(config.hooks.is_some());
        assert!(config.agents.is_some());
        assert!(config.skills.is_some());
        assert!(config.settings.is_some());

        // Check merge strategies
        assert_eq!(
            config.rules.as_ref().unwrap().merge_strategy,
            MergeStrategy::Concat
        );
        assert_eq!(
            config.commands.as_ref().unwrap().merge_strategy,
            MergeStrategy::Replace
        );
    }

    #[test]
    fn test_preset_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config = PresetConfig::default_preset("save-test");

        // Save
        config.save(temp_dir.path()).unwrap();

        // Verify file exists
        let config_file = temp_dir.path().join(".aidot-config.toml");
        assert!(config_file.exists());

        // Load
        let loaded = PresetConfig::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.metadata.name, "save-test");
        assert_eq!(loaded.metadata.version, "1.0.0");
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
            merge_strategy: MergeStrategy::Replace,
        };

        let toml = toml::to_string(&rules).unwrap();
        let deserialized: RulesSection = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.files.len(), 1);
        assert_eq!(deserialized.merge_strategy, MergeStrategy::Replace);
    }

    #[test]
    fn test_directory_section() {
        let section = DirectorySection {
            directory: "commands/".to_string(),
            merge_strategy: MergeStrategy::Concat,
        };

        let toml = toml::to_string(&section).unwrap();
        let deserialized: DirectorySection = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.directory, "commands/");
        assert_eq!(deserialized.merge_strategy, MergeStrategy::Concat);
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
