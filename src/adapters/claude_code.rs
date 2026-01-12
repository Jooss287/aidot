use super::traits::{ApplyResult, PreviewResult, TemplateFile, TemplateFiles, ToolAdapter};
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
    fn apply_rules(&self, files: &[TemplateFile], result: &mut ApplyResult) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let rules_dir = self.claude_dir().join("rules");
        fs::create_dir_all(&rules_dir)?;

        for file in files {
            // Preserve directory structure
            let target_path = rules_dir.join(&file.relative_path.replace("rules/", ""));

            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&target_path, &file.content)?;
            result.add_created(format!(".claude/rules/{}", file.relative_path.replace("rules/", "")));
        }

        Ok(())
    }

    /// Apply memory files: memory/*.md → .claude/CLAUDE.md (merged or replaced)
    fn apply_memory(&self, files: &[TemplateFile], strategy: &MergeStrategy, result: &mut ApplyResult) -> Result<()> {
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

        let action = if claude_md.exists() {
            match strategy {
                MergeStrategy::Concat => {
                    // Append to existing file
                    let existing = fs::read_to_string(&claude_md)?;
                    content = format!("{}\n\n---\n\n{}", existing, content);
                }
                MergeStrategy::Replace => {
                    // Replace entirely - content stays as is
                }
            }
            "updated"
        } else {
            "created"
        };

        fs::write(&claude_md, content)?;

        if action == "created" {
            result.add_created(".claude/CLAUDE.md".to_string());
        } else {
            result.add_updated(".claude/CLAUDE.md".to_string());
        }

        Ok(())
    }

    /// Apply commands: commands/*.md → .claude/commands/
    fn apply_commands(&self, files: &[TemplateFile], result: &mut ApplyResult) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let commands_dir = self.claude_dir().join("commands");
        fs::create_dir_all(&commands_dir)?;

        for file in files {
            let filename = file.relative_path.replace("commands/", "");
            let target_path = commands_dir.join(&filename);

            fs::write(&target_path, &file.content)?;
            result.add_created(format!(".claude/commands/{}", filename));
        }

        Ok(())
    }

    /// Apply MCP configs: mcp/*.json → .claude/settings.local.json (mcpServers section)
    fn apply_mcp(&self, files: &[TemplateFile], strategy: &MergeStrategy, result: &mut ApplyResult) -> Result<()> {
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
        fs::write(&settings_file, json_str)?;

        result.add_updated(".claude/settings.local.json".to_string());

        Ok(())
    }

    /// Apply hooks: hooks/*.json → .claude/hooks.json
    fn apply_hooks(&self, files: &[TemplateFile], result: &mut ApplyResult) -> Result<()> {
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
        fs::write(&hooks_file, json_str)?;

        result.add_created(".claude/hooks.json".to_string());

        Ok(())
    }

    /// Apply agents: agents/*.md → .claude/agents/
    fn apply_agents(&self, files: &[TemplateFile], result: &mut ApplyResult) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let agents_dir = self.claude_dir().join("agents");
        fs::create_dir_all(&agents_dir)?;

        for file in files {
            let filename = file.relative_path.replace("agents/", "");
            let target_path = agents_dir.join(&filename);

            fs::write(&target_path, &file.content)?;
            result.add_created(format!(".claude/agents/{}", filename));
        }

        Ok(())
    }

    /// Apply skills: skills/*.ts → .claude/skills/
    fn apply_skills(&self, files: &[TemplateFile], result: &mut ApplyResult) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let skills_dir = self.claude_dir().join("skills");
        fs::create_dir_all(&skills_dir)?;

        for file in files {
            let filename = file.relative_path.replace("skills/", "");
            let target_path = skills_dir.join(&filename);

            fs::write(&target_path, &file.content)?;
            result.add_created(format!(".claude/skills/{}", filename));
        }

        Ok(())
    }

    /// Apply settings: settings/*.json → .claude/settings.local.json (merged or replaced)
    fn apply_settings(&self, files: &[TemplateFile], strategy: &MergeStrategy, result: &mut ApplyResult) -> Result<()> {
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
        fs::write(&settings_file, json_str)?;

        result.add_updated(".claude/settings.local.json".to_string());

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
        _force: bool,
    ) -> Result<ApplyResult> {
        self.ensure_claude_dir()?;

        let mut result = ApplyResult::new();

        // Apply each section with their merge strategies
        self.apply_rules(&template_files.rules, &mut result)?;
        self.apply_memory(&template_files.memory, &template_files.memory_strategy, &mut result)?;
        self.apply_commands(&template_files.commands, &mut result)?;
        self.apply_mcp(&template_files.mcp, &template_files.mcp_strategy, &mut result)?;
        self.apply_hooks(&template_files.hooks, &mut result)?;
        self.apply_agents(&template_files.agents, &mut result)?;
        self.apply_skills(&template_files.skills, &mut result)?;
        self.apply_settings(&template_files.settings, &template_files.settings_strategy, &mut result)?;

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
