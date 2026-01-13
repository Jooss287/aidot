use crate::adapters::traits::{PresetFile, PresetFiles};
use crate::error::Result;
use crate::preset::PresetConfig;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Parse a preset repository directory
pub fn parse_preset(preset_dir: &Path) -> Result<(PresetConfig, PresetFiles)> {
    // Load configuration
    let config = PresetConfig::load(preset_dir)?;

    // Parse files from each section
    let mut preset_files = PresetFiles::default();

    // Parse rules
    if let Some(ref rules_section) = config.rules {
        preset_files.rules = parse_directory(preset_dir, "rules")?;
        preset_files.rules_strategy = rules_section.merge_strategy.clone();
    }

    // Parse memory
    if let Some(ref memory_section) = config.memory {
        preset_files.memory = parse_directory(preset_dir, "memory")?;
        preset_files.memory_strategy = memory_section.merge_strategy.clone();
    }

    // Parse commands
    if let Some(ref commands_section) = config.commands {
        preset_files.commands = parse_directory(preset_dir, "commands")?;
        preset_files.commands_strategy = commands_section.merge_strategy.clone();
    }

    // Parse MCP
    if let Some(ref mcp_section) = config.mcp {
        preset_files.mcp = parse_directory(preset_dir, "mcp")?;
        preset_files.mcp_strategy = mcp_section.merge_strategy.clone();
    }

    // Parse hooks
    if let Some(ref hooks_section) = config.hooks {
        preset_files.hooks = parse_directory(preset_dir, "hooks")?;
        preset_files.hooks_strategy = hooks_section.merge_strategy.clone();
    }

    // Parse agents
    if let Some(ref agents_section) = config.agents {
        preset_files.agents = parse_directory(preset_dir, "agents")?;
        preset_files.agents_strategy = agents_section.merge_strategy.clone();
    }

    // Parse skills
    if let Some(ref skills_section) = config.skills {
        preset_files.skills = parse_directory(preset_dir, "skills")?;
        preset_files.skills_strategy = skills_section.merge_strategy.clone();
    }

    // Parse settings
    if let Some(ref settings_section) = config.settings {
        preset_files.settings = parse_directory(preset_dir, "settings")?;
        preset_files.settings_strategy = settings_section.merge_strategy.clone();
    }

    Ok((config, preset_files))
}

/// Parse all files in a directory recursively
fn parse_directory(preset_dir: &Path, subdir: &str) -> Result<Vec<PresetFile>> {
    let target_dir = preset_dir.join(subdir);
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

        // Get relative path from preset root
        let relative_path = path
            .strip_prefix(preset_dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");

        files.push(PresetFile {
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
        let preset_dir = temp_dir.path();

        // Create test structure
        fs::create_dir_all(preset_dir.join("rules")).unwrap();
        fs::write(preset_dir.join("rules/test.md"), "# Test Rule").unwrap();

        let files = parse_directory(preset_dir, "rules").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "rules/test.md");
        assert_eq!(files[0].content, "# Test Rule");
    }
}
