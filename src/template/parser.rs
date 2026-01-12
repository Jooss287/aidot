use crate::adapters::traits::{TemplateFile, TemplateFiles};
use crate::error::Result;
use crate::template::TemplateConfig;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Parse a template repository directory
pub fn parse_template(template_dir: &Path) -> Result<(TemplateConfig, TemplateFiles)> {
    // Load configuration
    let config = TemplateConfig::load(template_dir)?;

    // Parse files from each section
    let mut template_files = TemplateFiles::default();

    // Parse rules
    if let Some(ref rules_section) = config.rules {
        template_files.rules = parse_directory(template_dir, "rules")?;
        template_files.rules_strategy = rules_section.merge_strategy.clone();
    }

    // Parse memory
    if let Some(ref memory_section) = config.memory {
        template_files.memory = parse_directory(template_dir, "memory")?;
        template_files.memory_strategy = memory_section.merge_strategy.clone();
    }

    // Parse commands
    if let Some(ref commands_section) = config.commands {
        template_files.commands = parse_directory(template_dir, "commands")?;
        template_files.commands_strategy = commands_section.merge_strategy.clone();
    }

    // Parse MCP
    if let Some(ref mcp_section) = config.mcp {
        template_files.mcp = parse_directory(template_dir, "mcp")?;
        template_files.mcp_strategy = mcp_section.merge_strategy.clone();
    }

    // Parse hooks
    if let Some(ref hooks_section) = config.hooks {
        template_files.hooks = parse_directory(template_dir, "hooks")?;
        template_files.hooks_strategy = hooks_section.merge_strategy.clone();
    }

    // Parse agents
    if let Some(ref agents_section) = config.agents {
        template_files.agents = parse_directory(template_dir, "agents")?;
        template_files.agents_strategy = agents_section.merge_strategy.clone();
    }

    // Parse skills
    if let Some(ref skills_section) = config.skills {
        template_files.skills = parse_directory(template_dir, "skills")?;
        template_files.skills_strategy = skills_section.merge_strategy.clone();
    }

    // Parse settings
    if let Some(ref settings_section) = config.settings {
        template_files.settings = parse_directory(template_dir, "settings")?;
        template_files.settings_strategy = settings_section.merge_strategy.clone();
    }

    Ok((config, template_files))
}

/// Parse all files in a directory recursively
fn parse_directory(template_dir: &Path, subdir: &str) -> Result<Vec<TemplateFile>> {
    let target_dir = template_dir.join(subdir);
    let mut files = Vec::new();

    if !target_dir.exists() {
        return Ok(files);
    }

    for entry in WalkDir::new(&target_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let content = fs::read_to_string(path)?;

        // Get relative path from template root
        let relative_path = path
            .strip_prefix(template_dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");

        files.push(TemplateFile {
            relative_path,
            content,
        });
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_directory() {
        let temp_dir = TempDir::new().unwrap();
        let template_dir = temp_dir.path();

        // Create test structure
        fs::create_dir_all(template_dir.join("rules")).unwrap();
        fs::write(
            template_dir.join("rules/test.md"),
            "# Test Rule",
        )
        .unwrap();

        let files = parse_directory(template_dir, "rules").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "rules/test.md");
        assert_eq!(files[0].content, "# Test Rule");
    }
}
