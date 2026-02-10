use super::traits::{
    strip_section_prefix, write_with_conflict, ApplyResult, ConflictMode, PresetFile, PresetFiles,
    ScanResult, ToolAdapter,
};
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

    /// Ensure .claude directory exists
    fn ensure_claude_dir(&self) -> Result<()> {
        let claude_dir = self.claude_dir();
        if !claude_dir.exists() {
            fs::create_dir_all(&claude_dir)?;
        }
        Ok(())
    }

    /// Apply rules files: rules/*.md → .claude/rules/
    fn apply_rules(
        &self,
        files: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let rules_dir = self.claude_dir().join("rules");
        fs::create_dir_all(&rules_dir)?;

        for file in files {
            let relative = strip_section_prefix(&file.relative_path, "rules");
            let target_path = rules_dir.join(&relative);
            let display_path = format!(".claude/rules/{}", relative);

            *mode = write_with_conflict(&target_path, &file.content, *mode, result, &display_path)?;
        }

        Ok(())
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

        *mode = write_with_conflict(&claude_md, &content, *mode, result, ".claude/CLAUDE.md")?;

        Ok(())
    }

    /// Apply commands: commands/*.md → .claude/commands/
    fn apply_commands(
        &self,
        files: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let commands_dir = self.claude_dir().join("commands");
        fs::create_dir_all(&commands_dir)?;

        for file in files {
            let filename = strip_section_prefix(&file.relative_path, "commands");
            let target_path = commands_dir.join(&filename);
            let display_path = format!(".claude/commands/{}", filename);

            *mode = write_with_conflict(&target_path, &file.content, *mode, result, &display_path)?;
        }

        Ok(())
    }

    /// Apply MCP configs: mcp/*.json → .claude/settings.local.json (mcpServers section)
    fn apply_mcp(
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

        // Ensure mcpServers object exists
        if settings.get("mcpServers").is_none() {
            settings["mcpServers"] = serde_json::json!({});
        }

        // Merge MCP configurations
        for file in files {
            let mcp_config: serde_json::Value = serde_json::from_str(&file.content)?;
            let server_name = strip_section_prefix(&file.relative_path, "mcp").replace(".json", "");

            settings["mcpServers"][server_name] = mcp_config;
        }

        let json_str = serde_json::to_string_pretty(&settings)?;
        *mode = write_with_conflict(
            &settings_file,
            &json_str,
            *mode,
            result,
            ".claude/settings.local.json",
        )?;

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
        *mode = write_with_conflict(&hooks_file, &json_str, *mode, result, ".claude/hooks.json")?;

        Ok(())
    }

    /// Apply agents: agents/*.md → .claude/agents/
    fn apply_agents(
        &self,
        files: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let agents_dir = self.claude_dir().join("agents");
        fs::create_dir_all(&agents_dir)?;

        for file in files {
            let filename = strip_section_prefix(&file.relative_path, "agents");
            let target_path = agents_dir.join(&filename);
            let display_path = format!(".claude/agents/{}", filename);

            *mode = write_with_conflict(&target_path, &file.content, *mode, result, &display_path)?;
        }

        Ok(())
    }

    /// Apply skills: skills/*.ts → .claude/skills/
    fn apply_skills(
        &self,
        files: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let skills_dir = self.claude_dir().join("skills");
        fs::create_dir_all(&skills_dir)?;

        for file in files {
            let filename = strip_section_prefix(&file.relative_path, "skills");
            let target_path = skills_dir.join(&filename);
            let display_path = format!(".claude/skills/{}", filename);

            *mode = write_with_conflict(&target_path, &file.content, *mode, result, &display_path)?;
        }

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
        *mode = write_with_conflict(
            &settings_file,
            &json_str,
            *mode,
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
        // Check if .claude directory exists or if 'claude' command is available
        let claude_dir = self.claude_dir();
        if claude_dir.exists() {
            return true;
        }

        // Check if claude command exists
        #[cfg(target_os = "windows")]
        let check_cmd = std::process::Command::new("where").arg("claude").output();

        #[cfg(not(target_os = "windows"))]
        let check_cmd = std::process::Command::new("which").arg("claude").output();

        check_cmd
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn scan(&self, preset_files: &PresetFiles, _target_dir: &Path) -> ScanResult {
        let mut result = ScanResult::new();
        let claude_md = self.claude_dir().join("CLAUDE.md");
        let settings_file = self.claude_dir().join("settings.local.json");

        // Rules
        for file in &preset_files.rules {
            let filename = strip_section_prefix(&file.relative_path, "rules");
            let target = format!(".claude/rules/{}", filename);
            let target_path = self.claude_dir().join("rules").join(&filename);
            result.add_change_with_content(
                target,
                "rules".to_string(),
                &target_path,
                &file.content,
            );
        }

        // Memory
        if !preset_files.memory.is_empty() {
            result.add_change(
                ".claude/CLAUDE.md".to_string(),
                "memory".to_string(),
                claude_md.exists(),
            );
        }

        // Commands
        for file in &preset_files.commands {
            let filename = strip_section_prefix(&file.relative_path, "commands");
            let target = format!(".claude/commands/{}", filename);
            let target_path = self.claude_dir().join("commands").join(&filename);
            result.add_change_with_content(
                target,
                "commands".to_string(),
                &target_path,
                &file.content,
            );
        }

        // MCP
        if !preset_files.mcp.is_empty() {
            result.add_change(
                ".claude/settings.local.json".to_string(),
                "mcp".to_string(),
                settings_file.exists(),
            );
        }

        // Hooks
        if !preset_files.hooks.is_empty() {
            let hooks_file = self.claude_dir().join("hooks.json");
            result.add_change(
                ".claude/hooks.json".to_string(),
                "hooks".to_string(),
                hooks_file.exists(),
            );
        }

        // Agents
        for file in &preset_files.agents {
            let filename = strip_section_prefix(&file.relative_path, "agents");
            let target = format!(".claude/agents/{}", filename);
            let target_path = self.claude_dir().join("agents").join(&filename);
            result.add_change_with_content(
                target,
                "agents".to_string(),
                &target_path,
                &file.content,
            );
        }

        // Skills
        for file in &preset_files.skills {
            let filename = strip_section_prefix(&file.relative_path, "skills");
            let target = format!(".claude/skills/{}", filename);
            let target_path = self.claude_dir().join("skills").join(&filename);
            result.add_change_with_content(
                target,
                "skills".to_string(),
                &target_path,
                &file.content,
            );
        }

        // Settings
        if !preset_files.settings.is_empty() {
            result.add_change(
                ".claude/settings.local.json".to_string(),
                "settings".to_string(),
                settings_file.exists(),
            );
        }

        result
    }

    fn apply(
        &self,
        preset_files: &PresetFiles,
        _target_dir: &Path,
        conflict_mode: ConflictMode,
    ) -> Result<ApplyResult> {
        self.ensure_claude_dir()?;

        let mut result = ApplyResult::new();
        let mut mode = conflict_mode;

        // Apply each section
        self.apply_rules(&preset_files.rules, &mut result, &mut mode)?;
        self.apply_memory(&preset_files.memory, &mut result, &mut mode)?;
        self.apply_commands(&preset_files.commands, &mut result, &mut mode)?;
        self.apply_mcp(&preset_files.mcp, &mut result, &mut mode)?;
        self.apply_hooks(&preset_files.hooks, &mut result, &mut mode)?;
        self.apply_agents(&preset_files.agents, &mut result, &mut mode)?;
        self.apply_skills(&preset_files.skills, &mut result, &mut mode)?;
        self.apply_settings(&preset_files.settings, &mut result, &mut mode)?;

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
            .apply(&preset_files, temp_dir.path(), ConflictMode::Force)
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
            .apply(&preset_files, temp_dir.path(), ConflictMode::Force)
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
            .apply(&preset_files, temp_dir.path(), ConflictMode::Force)
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
            .apply(&preset_files, temp_dir.path(), ConflictMode::Force)
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
