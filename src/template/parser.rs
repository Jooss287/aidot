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
    if config.rules.is_some() {
        template_files.rules = parse_directory(template_dir, "rules")?;
    }

    // Parse memory
    if config.memory.is_some() {
        template_files.memory = parse_directory(template_dir, "memory")?;
    }

    // Parse commands
    if config.commands.is_some() {
        template_files.commands = parse_directory(template_dir, "commands")?;
    }

    // Parse MCP
    if config.mcp.is_some() {
        template_files.mcp = parse_directory(template_dir, "mcp")?;
    }

    // Parse hooks
    if config.hooks.is_some() {
        template_files.hooks = parse_directory(template_dir, "hooks")?;
    }

    // Parse agents
    if config.agents.is_some() {
        template_files.agents = parse_directory(template_dir, "agents")?;
    }

    // Parse skills
    if config.skills.is_some() {
        template_files.skills = parse_directory(template_dir, "skills")?;
    }

    // Parse settings
    if config.settings.is_some() {
        template_files.settings = parse_directory(template_dir, "settings")?;
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
