use super::common::{
    apply_json_merge, apply_one_to_one, ensure_dir, scan_merged_section, scan_one_to_one,
};
use super::conflict::{write_with_conflict, ConflictMode};
use super::helpers::{is_command_available, strip_section_prefix};
use super::traits::{ApplyResult, PresetFile, PresetFiles, ScanResult, ToolAdapter};
use crate::error::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Claude Code adapter
pub struct ClaudeCodeAdapter {
    project_dir: PathBuf,
}

impl ClaudeCodeAdapter {
    pub fn new(project_dir: &Path) -> Self {
        Self {
            project_dir: project_dir.to_path_buf(),
        }
    }

    /// Get the .claude directory path
    fn claude_dir(&self) -> PathBuf {
        self.project_dir.join(".claude")
    }

    /// Apply memory files: memory/*.md → .claude/CLAUDE.md
    fn apply_memory(
        &self,
        files: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let claude_md = self.claude_dir().join("CLAUDE.md");

        // Merge all memory files from preset
        let mut content = String::new();
        for (i, file) in files.iter().enumerate() {
            if i > 0 {
                content.push_str("\n\n---\n\n");
            }
            content.push_str(&file.content);
        }

        write_with_conflict(&claude_md, &content, mode, result, ".claude/CLAUDE.md")?;

        Ok(())
    }

    /// Apply hooks: hooks/*.json → .claude/hooks.json
    fn apply_hooks(
        &self,
        files: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let hooks_file = self.claude_dir().join("hooks.json");

        // Merge all hooks into one JSON object
        let mut hooks = serde_json::Map::new();
        for file in files {
            let hook_name = strip_section_prefix(&file.relative_path, "hooks").replace(".json", "");
            let hook_config: serde_json::Value = serde_json::from_str(&file.content)?;
            hooks.insert(hook_name, hook_config);
        }

        let json_str = serde_json::to_string_pretty(&serde_json::Value::Object(hooks))?;
        write_with_conflict(&hooks_file, &json_str, mode, result, ".claude/hooks.json")?;

        Ok(())
    }

    /// Apply settings: settings/*.json → .claude/settings.local.json
    fn apply_settings(
        &self,
        files: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let settings_file = self.claude_dir().join("settings.local.json");

        // Read existing settings or create new
        let mut settings: serde_json::Value = if settings_file.exists() {
            let content = fs::read_to_string(&settings_file)?;
            serde_json::from_str(&content)?
        } else {
            serde_json::json!({})
        };

        // Merge all settings files
        for file in files {
            let new_settings: serde_json::Value = serde_json::from_str(&file.content)?;
            if let serde_json::Value::Object(new_map) = new_settings {
                if let serde_json::Value::Object(ref mut settings_map) = settings {
                    for (key, value) in new_map {
                        settings_map.insert(key, value);
                    }
                }
            }
        }

        let json_str = serde_json::to_string_pretty(&settings)?;
        write_with_conflict(
            &settings_file,
            &json_str,
            mode,
            result,
            ".claude/settings.local.json",
        )?;

        Ok(())
    }
}

impl ToolAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &str {
        "Claude Code"
    }

    fn detect(&self) -> bool {
        self.claude_dir().exists() || is_command_available("claude")
    }

    fn scan(&self, preset_files: &PresetFiles, _target_dir: &Path) -> ScanResult {
        let mut result = ScanResult::new();
        let claude_dir = self.claude_dir();
        let settings_file = claude_dir.join("settings.local.json");

        // 1:1 mapped sections
        scan_one_to_one(
            &preset_files.rules,
            "rules",
            &claude_dir.join("rules"),
            ".claude/rules",
            &mut result,
            None,
            None,
        );
        scan_one_to_one(
            &preset_files.commands,
            "commands",
            &claude_dir.join("commands"),
            ".claude/commands",
            &mut result,
            None,
            None,
        );
        scan_one_to_one(
            &preset_files.agents,
            "agents",
            &claude_dir.join("agents"),
            ".claude/agents",
            &mut result,
            None,
            None,
        );
        scan_one_to_one(
            &preset_files.skills,
            "skills",
            &claude_dir.join("skills"),
            ".claude/skills",
            &mut result,
            None,
            None,
        );

        // Merged sections
        if !preset_files.memory.is_empty() {
            result.add_change(
                ".claude/CLAUDE.md".to_string(),
                "memory".to_string(),
                claude_dir.join("CLAUDE.md").exists(),
            );
        }
        scan_merged_section(
            &preset_files.mcp,
            ".claude/settings.local.json",
            "mcp",
            &settings_file,
            &mut result,
        );
        scan_merged_section(
            &preset_files.hooks,
            ".claude/hooks.json",
            "hooks",
            &claude_dir.join("hooks.json"),
            &mut result,
        );
        scan_merged_section(
            &preset_files.settings,
            ".claude/settings.local.json",
            "settings",
            &settings_file,
            &mut result,
        );

        result
    }

    fn apply(
        &self,
        preset_files: &PresetFiles,
        _target_dir: &Path,
        conflict_mode: &mut ConflictMode,
    ) -> Result<ApplyResult> {
        ensure_dir(&self.claude_dir())?;

        let mut result = ApplyResult::new();
        let claude_dir = self.claude_dir();

        // Apply merged sections first (may trigger interactive prompts)
        self.apply_memory(&preset_files.memory, &mut result, conflict_mode)?;
        apply_json_merge(
            &preset_files.mcp,
            "mcp",
            &claude_dir.join("settings.local.json"),
            ".claude/settings.local.json",
            "mcpServers",
            serde_json::json!({}),
            &mut result,
            conflict_mode,
        )?;
        self.apply_hooks(&preset_files.hooks, &mut result, conflict_mode)?;
        self.apply_settings(&preset_files.settings, &mut result, conflict_mode)?;

        // 1:1 mapped sections (resolved immediately from PreResolved map)
        apply_one_to_one(
            &preset_files.rules,
            "rules",
            &claude_dir.join("rules"),
            ".claude/rules",
            &mut result,
            conflict_mode,
            None,
            None,
        )?;
        apply_one_to_one(
            &preset_files.commands,
            "commands",
            &claude_dir.join("commands"),
            ".claude/commands",
            &mut result,
            conflict_mode,
            None,
            None,
        )?;
        apply_one_to_one(
            &preset_files.agents,
            "agents",
            &claude_dir.join("agents"),
            ".claude/agents",
            &mut result,
            conflict_mode,
            None,
            None,
        )?;
        apply_one_to_one(
            &preset_files.skills,
            "skills",
            &claude_dir.join("skills"),
            ".claude/skills",
            &mut result,
            conflict_mode,
            None,
            None,
        )?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_adapter() -> (TempDir, ClaudeCodeAdapter) {
        let temp_dir = TempDir::new().unwrap();
        let adapter = ClaudeCodeAdapter::new(temp_dir.path());
        (temp_dir, adapter)
    }

    #[test]
    fn test_adapter_name() {
        let (_temp_dir, adapter) = create_test_adapter();
        assert_eq!(adapter.name(), "Claude Code");
    }

    #[test]
    fn test_detect_no_claude_dir() {
        let (temp_dir, _adapter) = create_test_adapter();
        let claude_dir = temp_dir.path().join(".claude");
        assert!(!claude_dir.exists());
    }

    #[test]
    fn test_detect_with_claude_dir() {
        let (temp_dir, adapter) = create_test_adapter();
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        assert!(adapter.detect());
    }

    #[test]
    fn test_apply_rules() {
        let (temp_dir, adapter) = create_test_adapter();

        let preset_files = PresetFiles {
            rules: vec![PresetFile {
                relative_path: "rules/code-style.md".to_string(),
                content: "# Code Style Rules".to_string(),
            }],
            ..Default::default()
        };

        let result = adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        assert_eq!(result.created.len(), 1);
        assert!(result.created[0].contains("code-style.md"));

        // Verify file was created
        let created_file = temp_dir.path().join(".claude/rules/code-style.md");
        assert!(created_file.exists());
        assert_eq!(
            fs::read_to_string(created_file).unwrap(),
            "# Code Style Rules"
        );
    }

    #[test]
    fn test_apply_memory_new_file() {
        let (temp_dir, adapter) = create_test_adapter();

        let preset_files = PresetFiles {
            memory: vec![PresetFile {
                relative_path: "memory/context.md".to_string(),
                content: "# Project Context".to_string(),
            }],
            ..Default::default()
        };

        let result = adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        assert!(result.created.iter().any(|f| f.contains("CLAUDE.md")));

        let claude_md = temp_dir.path().join(".claude/CLAUDE.md");
        assert!(claude_md.exists());
    }

    #[test]
    fn test_apply_memory_existing_file() {
        let (temp_dir, adapter) = create_test_adapter();

        // Create existing CLAUDE.md
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(
            temp_dir.path().join(".claude/CLAUDE.md"),
            "# Existing Content",
        )
        .unwrap();

        let preset_files = PresetFiles {
            memory: vec![PresetFile {
                relative_path: "memory/new.md".to_string(),
                content: "# New Content".to_string(),
            }],
            ..Default::default()
        };

        let result = adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        // Should update existing file
        assert!(result.updated.iter().any(|f| f.contains("CLAUDE.md")));
    }

    #[test]
    fn test_apply_commands() {
        let (temp_dir, adapter) = create_test_adapter();

        let preset_files = PresetFiles {
            commands: vec![PresetFile {
                relative_path: "commands/build.md".to_string(),
                content: "# Build Command".to_string(),
            }],
            ..Default::default()
        };

        let result = adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        assert!(result.created.iter().any(|f| f.contains("build.md")));

        let cmd_file = temp_dir.path().join(".claude/commands/build.md");
        assert!(cmd_file.exists());
    }

    #[test]
    fn test_scan_creates() {
        let (_temp_dir, adapter) = create_test_adapter();

        let preset_files = PresetFiles {
            rules: vec![PresetFile {
                relative_path: "rules/test.md".to_string(),
                content: "# Test".to_string(),
            }],
            memory: vec![PresetFile {
                relative_path: "memory/ctx.md".to_string(),
                content: "# Context".to_string(),
            }],
            ..Default::default()
        };

        let result = adapter.scan(&preset_files, Path::new("."));

        assert!(result.has_changes());
        assert!(!result.has_conflicts());
        assert_eq!(result.creates().len(), 2);
    }

    #[test]
    fn test_scan_conflicts() {
        let (temp_dir, adapter) = create_test_adapter();

        // Create existing file
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(temp_dir.path().join(".claude/CLAUDE.md"), "existing").unwrap();

        let preset_files = PresetFiles {
            memory: vec![PresetFile {
                relative_path: "memory/new.md".to_string(),
                content: "# New".to_string(),
            }],
            ..Default::default()
        };

        let result = adapter.scan(&preset_files, temp_dir.path());

        assert!(result.has_conflicts());
        assert_eq!(result.conflicts().len(), 1);
        assert!(result.conflicts()[0].path.contains("CLAUDE.md"));
    }
}
