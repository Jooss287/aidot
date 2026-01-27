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
    if config.rules.is_some() {
        preset_files.rules = parse_directory(preset_dir, "rules")?;
    }

    // Parse memory
    if config.memory.is_some() {
        preset_files.memory = parse_directory(preset_dir, "memory")?;
    }

    // Parse commands
    if config.commands.is_some() {
        preset_files.commands = parse_directory(preset_dir, "commands")?;
    }

    // Parse MCP
    if config.mcp.is_some() {
        preset_files.mcp = parse_directory(preset_dir, "mcp")?;
    }

    // Parse hooks
    if config.hooks.is_some() {
        preset_files.hooks = parse_directory(preset_dir, "hooks")?;
    }

    // Parse agents
    if config.agents.is_some() {
        preset_files.agents = parse_directory(preset_dir, "agents")?;
    }

    // Parse skills
    if config.skills.is_some() {
        preset_files.skills = parse_directory(preset_dir, "skills")?;
    }

    // Parse settings
    if config.settings.is_some() {
        preset_files.settings = parse_directory(preset_dir, "settings")?;
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
