use crate::adapters::traits::{PresetFile, PresetFiles};
use crate::error::{AidotError, Result};
use crate::preset::PresetConfig;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Reserved directories that cannot be placed in root/
/// These should be managed through dedicated sections (rules/, memory/, etc.)
const RESERVED_DIRS: &[&str] = &[".claude", ".cursor", ".github", ".vscode"];

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

    // Parse root files
    if config.root.is_some() {
        preset_files.root = parse_root_directory(preset_dir, "root")?;
    }

    Ok((config, preset_files))
}

/// Parse root directory files and validate they don't contain reserved directories
fn parse_root_directory(preset_dir: &Path, subdir: &str) -> Result<Vec<PresetFile>> {
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

        // Get relative path from root/ directory (not from preset root)
        let relative_from_root = path
            .strip_prefix(&target_dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");

        // Validate: check if path starts with reserved directories
        for reserved in RESERVED_DIRS {
            if relative_from_root.starts_with(reserved) {
                return Err(AidotError::InvalidPreset(format!(
                    "root/ cannot contain '{}/' - use dedicated sections instead\n\
                     Hint: Move {} files to the appropriate sections:\n  \
                     - rules/*.md   → rules/\n  \
                     - memory/*.md  → memory/\n  \
                     - commands/    → commands/\n  \
                     - mcp/*.json   → mcp/",
                    reserved, reserved
                )));
            }
        }

        files.push(PresetFile {
            relative_path: relative_from_root,
            content,
        });
    }

    Ok(files)
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

    #[test]
    fn test_parse_root_directory() {
        let temp_dir = TempDir::new().unwrap();
        let preset_dir = temp_dir.path();

        // Create root directory with valid files
        fs::create_dir_all(preset_dir.join("root")).unwrap();
        fs::write(
            preset_dir.join("root/.editorconfig"),
            "[*]\nindent_size = 4",
        )
        .unwrap();
        fs::write(preset_dir.join("root/.prettierrc"), "{}").unwrap();

        let files = parse_root_directory(preset_dir, "root").unwrap();
        assert_eq!(files.len(), 2);

        // Check that relative paths don't include "root/" prefix
        let paths: Vec<_> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert!(paths.contains(&".editorconfig"));
        assert!(paths.contains(&".prettierrc"));
    }

    #[test]
    fn test_parse_root_directory_nested() {
        let temp_dir = TempDir::new().unwrap();
        let preset_dir = temp_dir.path();

        // Create nested structure in root
        fs::create_dir_all(preset_dir.join("root/config")).unwrap();
        fs::write(preset_dir.join("root/config/settings.json"), "{}").unwrap();

        let files = parse_root_directory(preset_dir, "root").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "config/settings.json");
    }

    #[test]
    fn test_parse_root_directory_rejects_reserved_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let preset_dir = temp_dir.path();

        // Create root directory with reserved .claude folder
        fs::create_dir_all(preset_dir.join("root/.claude")).unwrap();
        fs::write(preset_dir.join("root/.claude/rules.md"), "# Rules").unwrap();

        let result = parse_root_directory(preset_dir, "root");
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains(".claude"));
        assert!(err_msg.contains("use dedicated sections"));
    }

    #[test]
    fn test_parse_root_directory_rejects_cursor() {
        let temp_dir = TempDir::new().unwrap();
        let preset_dir = temp_dir.path();

        fs::create_dir_all(preset_dir.join("root/.cursor")).unwrap();
        fs::write(preset_dir.join("root/.cursor/config.json"), "{}").unwrap();

        let result = parse_root_directory(preset_dir, "root");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".cursor"));
    }

    #[test]
    fn test_parse_root_directory_rejects_github() {
        let temp_dir = TempDir::new().unwrap();
        let preset_dir = temp_dir.path();

        fs::create_dir_all(preset_dir.join("root/.github")).unwrap();
        fs::write(preset_dir.join("root/.github/copilot.md"), "# Copilot").unwrap();

        let result = parse_root_directory(preset_dir, "root");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".github"));
    }

    #[test]
    fn test_parse_root_directory_rejects_vscode() {
        let temp_dir = TempDir::new().unwrap();
        let preset_dir = temp_dir.path();

        fs::create_dir_all(preset_dir.join("root/.vscode")).unwrap();
        fs::write(preset_dir.join("root/.vscode/settings.json"), "{}").unwrap();

        let result = parse_root_directory(preset_dir, "root");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".vscode"));
    }
}
