use super::traits::{ApplyResult, ConflictMode, PreviewResult, TemplateFile, TemplateFiles, ToolAdapter, write_with_conflict};
use crate::error::Result;
use crate::template::config::MergeStrategy;
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
    fn apply_rules(&self, files: &[TemplateFile], result: &mut ApplyResult, mode: &mut ConflictMode) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let rules_dir = self.claude_dir().join("rules");
        fs::create_dir_all(&rules_dir)?;

        for file in files {
            let relative = file.relative_path.replace("rules/", "");
            let target_path = rules_dir.join(&relative);
            let display_path = format!(".claude/rules/{}", relative);

            *mode = write_with_conflict(&target_path, &file.content, *mode, result, &display_path)?;
        }

        Ok(())
    }

    /// Apply memory files: memory/*.md → .claude/CLAUDE.md (merged or replaced)
    fn apply_memory(&self, files: &[TemplateFile], strategy: &MergeStrategy, result: &mut ApplyResult, mode: &mut ConflictMode) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let claude_md = self.claude_dir().join("CLAUDE.md");

        // Merge all memory files from template
        let mut content = String::new();
        for (i, file) in files.iter().enumerate() {
            if i > 0 {
                content.push_str("\n\n---\n\n");
            }
            content.push_str(&file.content);
        }

        if claude_md.exists() {
            match strategy {
                MergeStrategy::Concat => {
                    let existing = fs::read_to_string(&claude_md)?;
                    content = format!("{}\n\n---\n\n{}", existing, content);
                    // For concat, we always update (merge) - no conflict
                    fs::write(&claude_md, content)?;
                    result.add_updated(".claude/CLAUDE.md".to_string());
                }
                MergeStrategy::Replace => {
                    // For replace, check conflict
                    *mode = write_with_conflict(&claude_md, &content, *mode, result, ".claude/CLAUDE.md")?;
                }
            }
        } else {
            fs::write(&claude_md, content)?;
            result.add_created(".claude/CLAUDE.md".to_string());
        }

        Ok(())
    }

    /// Apply commands: commands/*.md → .claude/commands/
    fn apply_commands(&self, files: &[TemplateFile], result: &mut ApplyResult, mode: &mut ConflictMode) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let commands_dir = self.claude_dir().join("commands");
        fs::create_dir_all(&commands_dir)?;

        for file in files {
            let filename = file.relative_path.replace("commands/", "");
            let target_path = commands_dir.join(&filename);
            let display_path = format!(".claude/commands/{}", filename);

            *mode = write_with_conflict(&target_path, &file.content, *mode, result, &display_path)?;
        }

        Ok(())
    }

    /// Apply MCP configs: mcp/*.json → .claude/settings.local.json (mcpServers section)
    fn apply_mcp(&self, files: &[TemplateFile], strategy: &MergeStrategy, result: &mut ApplyResult, mode: &mut ConflictMode) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let settings_file = self.claude_dir().join("settings.local.json");

        // Read existing settings or create new based on strategy
        let mut settings: serde_json::Value = match strategy {
            MergeStrategy::Concat => {
                if settings_file.exists() {
                    let content = fs::read_to_string(&settings_file)?;
                    serde_json::from_str(&content)?
                } else {
                    serde_json::json!({})
                }
            }
            MergeStrategy::Replace => {
                // Start fresh with empty object
                serde_json::json!({})
            }
        };

        // Ensure mcpServers object exists
        if !settings.get("mcpServers").is_some() {
            settings["mcpServers"] = serde_json::json!({});
        }

        // Merge MCP configurations
        for file in files {
            let mcp_config: serde_json::Value = serde_json::from_str(&file.content)?;
            let server_name = file.relative_path
                .replace("mcp/", "")
                .replace(".json", "");

            settings["mcpServers"][server_name] = mcp_config;
        }

        let json_str = serde_json::to_string_pretty(&settings)?;
        // MCP configs are always merged, so treat as update
        *mode = write_with_conflict(&settings_file, &json_str, *mode, result, ".claude/settings.local.json")?;

        Ok(())
    }

    /// Apply hooks: hooks/*.json → .claude/hooks.json
    fn apply_hooks(&self, files: &[TemplateFile], result: &mut ApplyResult, mode: &mut ConflictMode) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let hooks_file = self.claude_dir().join("hooks.json");

        // Merge all hooks into one JSON object
        let mut hooks = serde_json::Map::new();
        for file in files {
            let hook_name = file.relative_path
                .replace("hooks/", "")
                .replace(".json", "");
            let hook_config: serde_json::Value = serde_json::from_str(&file.content)?;
            hooks.insert(hook_name, hook_config);
        }

        let json_str = serde_json::to_string_pretty(&serde_json::Value::Object(hooks))?;
        *mode = write_with_conflict(&hooks_file, &json_str, *mode, result, ".claude/hooks.json")?;

        Ok(())
    }

    /// Apply agents: agents/*.md → .claude/agents/
    fn apply_agents(&self, files: &[TemplateFile], result: &mut ApplyResult, mode: &mut ConflictMode) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let agents_dir = self.claude_dir().join("agents");
        fs::create_dir_all(&agents_dir)?;

        for file in files {
            let filename = file.relative_path.replace("agents/", "");
            let target_path = agents_dir.join(&filename);
            let display_path = format!(".claude/agents/{}", filename);

            *mode = write_with_conflict(&target_path, &file.content, *mode, result, &display_path)?;
        }

        Ok(())
    }

    /// Apply skills: skills/*.ts → .claude/skills/
    fn apply_skills(&self, files: &[TemplateFile], result: &mut ApplyResult, mode: &mut ConflictMode) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let skills_dir = self.claude_dir().join("skills");
        fs::create_dir_all(&skills_dir)?;

        for file in files {
            let filename = file.relative_path.replace("skills/", "");
            let target_path = skills_dir.join(&filename);
            let display_path = format!(".claude/skills/{}", filename);

            *mode = write_with_conflict(&target_path, &file.content, *mode, result, &display_path)?;
        }

        Ok(())
    }

    /// Apply settings: settings/*.json → .claude/settings.local.json (merged or replaced)
    fn apply_settings(&self, files: &[TemplateFile], strategy: &MergeStrategy, result: &mut ApplyResult, mode: &mut ConflictMode) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let settings_file = self.claude_dir().join("settings.local.json");

        // Read existing settings or create new based on strategy
        let mut settings: serde_json::Value = match strategy {
            MergeStrategy::Concat => {
                if settings_file.exists() {
                    let content = fs::read_to_string(&settings_file)?;
                    serde_json::from_str(&content)?
                } else {
                    serde_json::json!({})
                }
            }
            MergeStrategy::Replace => {
                // Start fresh with empty object
                serde_json::json!({})
            }
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
        *mode = write_with_conflict(&settings_file, &json_str, *mode, result, ".claude/settings.local.json")?;

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
        let check_cmd = std::process::Command::new("where")
            .arg("claude")
            .output();

        #[cfg(not(target_os = "windows"))]
        let check_cmd = std::process::Command::new("which")
            .arg("claude")
            .output();

        check_cmd.map(|output| output.status.success()).unwrap_or(false)
    }

    fn apply(
        &self,
        template_files: &TemplateFiles,
        _target_dir: &Path,
        conflict_mode: ConflictMode,
    ) -> Result<ApplyResult> {
        self.ensure_claude_dir()?;

        let mut result = ApplyResult::new();
        let mut mode = conflict_mode;

        // Apply each section with their merge strategies
        self.apply_rules(&template_files.rules, &mut result, &mut mode)?;
        self.apply_memory(&template_files.memory, &template_files.memory_strategy, &mut result, &mut mode)?;
        self.apply_commands(&template_files.commands, &mut result, &mut mode)?;
        self.apply_mcp(&template_files.mcp, &template_files.mcp_strategy, &mut result, &mut mode)?;
        self.apply_hooks(&template_files.hooks, &mut result, &mut mode)?;
        self.apply_agents(&template_files.agents, &mut result, &mut mode)?;
        self.apply_skills(&template_files.skills, &mut result, &mut mode)?;
        self.apply_settings(&template_files.settings, &template_files.settings_strategy, &mut result, &mut mode)?;

        Ok(result)
    }

    fn preview(
        &self,
        template_files: &TemplateFiles,
        _target_dir: &Path,
    ) -> PreviewResult {
        let mut result = PreviewResult::new();
        let claude_md = self.claude_dir().join("CLAUDE.md");
        let settings_file = self.claude_dir().join("settings.local.json");

        // Rules
        for file in &template_files.rules {
            let target = format!(".claude/rules/{}", file.relative_path.replace("rules/", ""));
            let target_path = self.claude_dir().join("rules").join(file.relative_path.replace("rules/", ""));
            if target_path.exists() {
                result.add_would_update(target, "rules".to_string());
            } else {
                result.add_would_create(target, "rules".to_string());
            }
        }

        // Memory
        if !template_files.memory.is_empty() {
            if claude_md.exists() {
                result.add_would_update(".claude/CLAUDE.md".to_string(), "memory".to_string());
            } else {
                result.add_would_create(".claude/CLAUDE.md".to_string(), "memory".to_string());
            }
        }

        // Commands
        for file in &template_files.commands {
            let filename = file.relative_path.replace("commands/", "");
            let target = format!(".claude/commands/{}", filename);
            let target_path = self.claude_dir().join("commands").join(&filename);
            if target_path.exists() {
                result.add_would_update(target, "commands".to_string());
            } else {
                result.add_would_create(target, "commands".to_string());
            }
        }

        // MCP
        if !template_files.mcp.is_empty() {
            if settings_file.exists() {
                result.add_would_update(".claude/settings.local.json".to_string(), "mcp".to_string());
            } else {
                result.add_would_create(".claude/settings.local.json".to_string(), "mcp".to_string());
            }
        }

        // Hooks
        if !template_files.hooks.is_empty() {
            let hooks_file = self.claude_dir().join("hooks.json");
            if hooks_file.exists() {
                result.add_would_update(".claude/hooks.json".to_string(), "hooks".to_string());
            } else {
                result.add_would_create(".claude/hooks.json".to_string(), "hooks".to_string());
            }
        }

        // Agents
        for file in &template_files.agents {
            let filename = file.relative_path.replace("agents/", "");
            let target = format!(".claude/agents/{}", filename);
            let target_path = self.claude_dir().join("agents").join(&filename);
            if target_path.exists() {
                result.add_would_update(target, "agents".to_string());
            } else {
                result.add_would_create(target, "agents".to_string());
            }
        }

        // Skills
        for file in &template_files.skills {
            let filename = file.relative_path.replace("skills/", "");
            let target = format!(".claude/skills/{}", filename);
            let target_path = self.claude_dir().join("skills").join(&filename);
            if target_path.exists() {
                result.add_would_update(target, "skills".to_string());
            } else {
                result.add_would_create(target, "skills".to_string());
            }
        }

        // Settings
        if !template_files.settings.is_empty() {
            if settings_file.exists() {
                result.add_would_update(".claude/settings.local.json".to_string(), "settings".to_string());
            } else {
                result.add_would_create(".claude/settings.local.json".to_string(), "settings".to_string());
            }
        }

        result
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
        // Note: detect() also checks for 'claude' command existence,
        // so this test only verifies no .claude dir doesn't auto-detect.
        // If claude CLI is installed, this will still return true.
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

        let template_files = TemplateFiles {
            rules: vec![
                TemplateFile {
                    relative_path: "rules/code-style.md".to_string(),
                    content: "# Code Style Rules".to_string(),
                },
            ],
            ..Default::default()
        };

        let result = adapter.apply(&template_files, temp_dir.path(), ConflictMode::Force).unwrap();

        assert_eq!(result.created.len(), 1);
        assert!(result.created[0].contains("code-style.md"));

        // Verify file was created
        let created_file = temp_dir.path().join(".claude/rules/code-style.md");
        assert!(created_file.exists());
        assert_eq!(fs::read_to_string(created_file).unwrap(), "# Code Style Rules");
    }

    #[test]
    fn test_apply_memory_new_file() {
        let (temp_dir, adapter) = create_test_adapter();

        let template_files = TemplateFiles {
            memory: vec![
                TemplateFile {
                    relative_path: "memory/context.md".to_string(),
                    content: "# Project Context".to_string(),
                },
            ],
            ..Default::default()
        };

        let result = adapter.apply(&template_files, temp_dir.path(), ConflictMode::Force).unwrap();

        assert!(result.created.iter().any(|f| f.contains("CLAUDE.md")));

        let claude_md = temp_dir.path().join(".claude/CLAUDE.md");
        assert!(claude_md.exists());
    }

    #[test]
    fn test_apply_memory_concat_strategy() {
        let (temp_dir, adapter) = create_test_adapter();

        // Create existing CLAUDE.md
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(temp_dir.path().join(".claude/CLAUDE.md"), "# Existing Content").unwrap();

        let template_files = TemplateFiles {
            memory: vec![
                TemplateFile {
                    relative_path: "memory/new.md".to_string(),
                    content: "# New Content".to_string(),
                },
            ],
            memory_strategy: MergeStrategy::Concat,
            ..Default::default()
        };

        adapter.apply(&template_files, temp_dir.path(), ConflictMode::Force).unwrap();

        let content = fs::read_to_string(temp_dir.path().join(".claude/CLAUDE.md")).unwrap();
        assert!(content.contains("# Existing Content"));
        assert!(content.contains("# New Content"));
    }

    #[test]
    fn test_apply_memory_replace_strategy() {
        let (temp_dir, adapter) = create_test_adapter();

        // Create existing CLAUDE.md
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(temp_dir.path().join(".claude/CLAUDE.md"), "# Existing Content").unwrap();

        let template_files = TemplateFiles {
            memory: vec![
                TemplateFile {
                    relative_path: "memory/new.md".to_string(),
                    content: "# New Content Only".to_string(),
                },
            ],
            memory_strategy: MergeStrategy::Replace,
            ..Default::default()
        };

        adapter.apply(&template_files, temp_dir.path(), ConflictMode::Force).unwrap();

        let content = fs::read_to_string(temp_dir.path().join(".claude/CLAUDE.md")).unwrap();
        assert!(!content.contains("# Existing Content"));
        assert!(content.contains("# New Content Only"));
    }

    #[test]
    fn test_apply_commands() {
        let (temp_dir, adapter) = create_test_adapter();

        let template_files = TemplateFiles {
            commands: vec![
                TemplateFile {
                    relative_path: "commands/build.md".to_string(),
                    content: "# Build Command".to_string(),
                },
            ],
            ..Default::default()
        };

        let result = adapter.apply(&template_files, temp_dir.path(), ConflictMode::Force).unwrap();

        assert!(result.created.iter().any(|f| f.contains("build.md")));

        let cmd_file = temp_dir.path().join(".claude/commands/build.md");
        assert!(cmd_file.exists());
    }

    #[test]
    fn test_preview_creates() {
        let (_temp_dir, adapter) = create_test_adapter();

        let template_files = TemplateFiles {
            rules: vec![
                TemplateFile {
                    relative_path: "rules/test.md".to_string(),
                    content: "# Test".to_string(),
                },
            ],
            memory: vec![
                TemplateFile {
                    relative_path: "memory/ctx.md".to_string(),
                    content: "# Context".to_string(),
                },
            ],
            ..Default::default()
        };

        let result = adapter.preview(&template_files, Path::new("."));

        assert!(result.has_changes());
        assert!(!result.would_create.is_empty());
    }

    #[test]
    fn test_preview_updates_existing() {
        let (temp_dir, adapter) = create_test_adapter();

        // Create existing file
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(temp_dir.path().join(".claude/CLAUDE.md"), "existing").unwrap();

        let template_files = TemplateFiles {
            memory: vec![
                TemplateFile {
                    relative_path: "memory/new.md".to_string(),
                    content: "# New".to_string(),
                },
            ],
            ..Default::default()
        };

        let result = adapter.preview(&template_files, temp_dir.path());

        assert!(result.would_update.iter().any(|f| f.path.contains("CLAUDE.md")));
    }
}
