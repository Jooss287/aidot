use crate::error::{AidotError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Template configuration from .aidot-config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    Concat,
    Replace,
}

fn default_merge_strategy() -> MergeStrategy {
    MergeStrategy::Concat
}

impl TemplateConfig {
    /// Load template configuration from .aidot-config.toml
    #[allow(dead_code)]
    pub fn load(path: &Path) -> Result<Self> {
        let config_file = path.join(".aidot-config.toml");

        if !config_file.exists() {
            return Err(AidotError::InvalidTemplate(format!(
                "Missing .aidot-config.toml in {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(&config_file)?;
        let config: TemplateConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save template configuration to .aidot-config.toml
    pub fn save(&self, path: &Path) -> Result<()> {
        let config_file = path.join(".aidot-config.toml");
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_file, content)?;
        Ok(())
    }

    /// Create a default template configuration
    pub fn default_template(name: &str) -> Self {
        TemplateConfig {
            metadata: Metadata {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: Some("LLM configuration template".to_string()),
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

    #[test]
    fn test_default_template() {
        let config = TemplateConfig::default_template("test-template");
        assert_eq!(config.metadata.name, "test-template");
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
    fn test_template_config_serialization() {
        let config = TemplateConfig::default_template("test");
        let toml = toml::to_string_pretty(&config).unwrap();
        let deserialized: TemplateConfig = toml::from_str(&toml).unwrap();

        assert_eq!(config.metadata.name, deserialized.metadata.name);
    }
}
