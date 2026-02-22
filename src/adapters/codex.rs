use super::common::{apply_one_to_one, ensure_dir, scan_merged_section, scan_one_to_one};
use super::conflict::{write_with_conflict, ConflictMode};
use super::helpers::{has_frontmatter, is_command_available, strip_section_prefix};
use super::traits::{ApplyResult, PresetFile, PresetFiles, ScanResult, ToolAdapter};
use crate::error::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// OpenAI Codex CLI adapter
///
/// Converts presets to Codex CLI format:
/// - rules/*.md     → .codex/AGENTS.md (merged with --- separator)
/// - memory/*.md    → .codex/AGENTS.md (merged with rules)
/// - commands/*.md  → .codex/skills/<name>/SKILL.md (YAML front matter auto-generated)
/// - skills/*.md    → .codex/skills/<name>/SKILL.md (YAML front matter preserved if exists)
/// - agents/*.md    → .codex/agents/<name>.md + .codex/config.toml [agents.<name>]
/// - mcp/*.json     → .codex/config.toml [mcp_servers.*] (JSON → TOML)
/// - settings/*.json → unsupported (warning)
/// - hooks/*.json   → unsupported (silently skipped)
pub struct CodexAdapter {
    project_dir: PathBuf,
}

impl CodexAdapter {
    pub fn new(project_dir: &Path) -> Self {
        Self {
            project_dir: project_dir.to_path_buf(),
        }
    }

    fn codex_dir(&self) -> PathBuf {
        self.project_dir.join(".codex")
    }

    /// .codex/AGENTS.md (rules + memory merged here)
    fn agents_md(&self) -> PathBuf {
        self.codex_dir().join("AGENTS.md")
    }

    fn codex_skills_dir(&self) -> PathBuf {
        self.codex_dir().join("skills")
    }

    fn codex_agents_dir(&self) -> PathBuf {
        self.codex_dir().join("agents")
    }

    fn codex_config_toml(&self) -> PathBuf {
        self.codex_dir().join("config.toml")
    }

    /// Apply rules + memory → .codex/AGENTS.md (merged)
    ///
    /// `.codex/rules/*.rules` is Starlark sandbox policy, NOT markdown instructions.
    /// All custom instructions go into `.codex/AGENTS.md`.
    fn apply_agents_md(
        &self,
        rules: &[PresetFile],
        memory: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if rules.is_empty() && memory.is_empty() {
            return Ok(());
        }

        let agents_md = self.agents_md();

        let mut content = String::new();
        let mut first = true;
        for file in rules.iter().chain(memory.iter()) {
            if !first {
                content.push_str("\n\n---\n\n");
            }
            content.push_str(&file.content);
            first = false;
        }

        write_with_conflict(&agents_md, &content, mode, result, ".codex/AGENTS.md")?;

        Ok(())
    }

    /// Apply skills/commands: *.md → .codex/skills/<name>/...
    ///
    /// Each top-level entry gets its own subdirectory under .codex/skills/.
    /// Nested files preserve their relative path within the skill directory.
    /// YAML front matter `name` field is ensured on every SKILL.md.
    fn apply_skill_files(
        &self,
        files: &[PresetFile],
        section: &str,
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        for file in files {
            let stripped = strip_section_prefix(&file.relative_path, section);
            let (skill_name, relative_path) = Self::parse_skill_path(&stripped);

            let content = Self::ensure_frontmatter(&skill_name, section, &file.content);

            let target_path = self
                .codex_skills_dir()
                .join(&skill_name)
                .join(&relative_path);
            let display_path = format!(".codex/skills/{}/{}", skill_name, relative_path);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            write_with_conflict(&target_path, &content, mode, result, &display_path)?;
        }

        Ok(())
    }

    /// Scan skills/commands files for changes
    fn scan_skill_files(&self, files: &[PresetFile], section: &str, result: &mut ScanResult) {
        for file in files {
            let stripped = strip_section_prefix(&file.relative_path, section);
            let (skill_name, relative_path) = Self::parse_skill_path(&stripped);

            let content = Self::ensure_frontmatter(&skill_name, section, &file.content);

            let target_path = self
                .codex_skills_dir()
                .join(&skill_name)
                .join(&relative_path);
            let display_path = format!(".codex/skills/{}/{}", skill_name, relative_path);

            result.add_change_with_content(
                display_path,
                section.to_string(),
                &target_path,
                &content,
            );
        }
    }

    /// Parse a stripped skill path into (skill_name, relative_path_within_skill_dir).
    ///
    /// - Flat file: `"build.md"` → `("build", "SKILL.md")`
    /// - Subdirectory: `"my-skill/SKILL.md"` → `("my-skill", "SKILL.md")`
    /// - Nested: `"my-skill/sub/SKILL.md"` → `("my-skill", "sub/SKILL.md")`
    fn parse_skill_path(stripped: &str) -> (String, String) {
        if stripped.contains('/') || stripped.contains('\\') {
            let pos = stripped.find('/').or_else(|| stripped.find('\\')).unwrap();
            let name = stripped[..pos].to_string();
            let rest = stripped[pos + 1..].to_string();
            (name, rest)
        } else {
            // Flat file: "build.md" → skill "build", renamed to "SKILL.md"
            let stem = stripped.strip_suffix(".md").unwrap_or(stripped);
            (stem.to_string(), "SKILL.md".to_string())
        }
    }

    /// Apply mcp and agents to .codex/config.toml
    ///
    /// - mcp/*.json  → [mcp_servers.*] (JSON → TOML)
    /// - agents/*.md → [agents.*] (role extracted from H1 or filename)
    fn apply_config_toml(
        &self,
        mcp_files: &[PresetFile],
        agent_files: &[PresetFile],
        result: &mut ApplyResult,
        mode: &mut ConflictMode,
    ) -> Result<()> {
        if mcp_files.is_empty() && agent_files.is_empty() {
            return Ok(());
        }

        let config_path = self.codex_config_toml();

        // Load existing config.toml or start with empty table
        let mut config: toml::Table = if config_path.exists() {
            let raw = fs::read_to_string(&config_path)?;
            toml::from_str(&raw).unwrap_or_default()
        } else {
            toml::Table::new()
        };

        // Build [mcp_servers.*] section
        if !mcp_files.is_empty() {
            let mcp_servers = config
                .entry("mcp_servers".to_string())
                .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                .as_table_mut()
                .ok_or_else(|| {
                    crate::error::AidotError::InvalidInput("mcp_servers is not a table".to_string())
                })?;

            for file in mcp_files {
                let server_name =
                    strip_section_prefix(&file.relative_path, "mcp").replace(".json", "");
                let json_val: serde_json::Value = serde_json::from_str(&file.content)?;
                let toml_entry = json_value_to_toml_table(&json_val)?;
                mcp_servers.insert(server_name, toml::Value::Table(toml_entry));
            }
        }

        // Build [agents.*] section
        if !agent_files.is_empty() {
            let agents = config
                .entry("agents".to_string())
                .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                .as_table_mut()
                .ok_or_else(|| {
                    crate::error::AidotError::InvalidInput("agents is not a table".to_string())
                })?;

            for file in agent_files {
                let stripped = strip_section_prefix(&file.relative_path, "agents");
                let agent_name = stripped.strip_suffix(".md").unwrap_or(&stripped);
                let role = extract_h1(&file.content).unwrap_or_else(|| agent_name.to_string());

                let mut agent_entry = toml::Table::new();
                agent_entry.insert("role".to_string(), toml::Value::String(role));
                agents.insert(agent_name.to_string(), toml::Value::Table(agent_entry));
            }
        }

        let toml_str = toml::to_string_pretty(&config)?;
        write_with_conflict(&config_path, &toml_str, mode, result, ".codex/config.toml")?;

        Ok(())
    }

    /// Ensure YAML front matter with required `name` and `description` fields.
    ///
    /// Three cases:
    /// 1. No front matter → generate full front matter (name + description)
    /// 2. Front matter exists with `name` → return unchanged
    /// 3. Front matter exists without `name` → inject `name` and `description`
    ///
    /// For `commands` section, description uses "Explicit command file for {name}."
    /// For other sections, description is extracted from the first H1 heading.
    fn ensure_frontmatter(stem: &str, section: &str, content: &str) -> String {
        let make_description = |name: &str| -> String {
            if section == "commands" {
                format!("Explicit command file for {}.", name)
            } else {
                extract_h1(content).unwrap_or_else(|| name.to_string())
            }
        };

        if !has_frontmatter(content) {
            // Case 1: no front matter at all
            let description = make_description(stem);
            let frontmatter = format!("---\nname: {}\ndescription: {}\n---\n", stem, description);
            return format!("{}{}", frontmatter, content);
        }

        // Front matter exists — check if `name` field is present
        if let Some(fm_end) = content.find("\n---").map(|pos| pos + 4) {
            let fm_body = &content[..fm_end];
            let has_name = fm_body
                .lines()
                .any(|line| line.starts_with("name:") || line.starts_with("name :"));
            if has_name {
                // Case 2: has name, return unchanged
                return content.to_string();
            }

            // Case 3: front matter exists but no `name` field — inject name + description
            let body_after_fm = &content[fm_end..];
            let description = make_description(stem);

            // Reconstruct: keep existing fields, prepend name and description
            // fm_body starts with "---\n...", strip the opening "---\n"
            let existing_fields = fm_body
                .strip_prefix("---\n")
                .unwrap_or(fm_body)
                .strip_suffix("\n---")
                .unwrap_or(fm_body);

            return format!(
                "---\nname: {}\ndescription: {}\n{}\n---\n{}",
                stem,
                description,
                existing_fields.trim(),
                body_after_fm.trim_start_matches('\n')
            );
        }

        content.to_string()
    }
}

/// Extract the first H1 heading text from markdown content.
fn extract_h1(content: &str) -> Option<String> {
    content
        .lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_string())
}

/// Recursively convert a serde_json::Value object to a toml::Table.
fn json_value_to_toml_table(json: &serde_json::Value) -> Result<toml::Table> {
    let mut table = toml::Table::new();
    if let serde_json::Value::Object(map) = json {
        for (key, val) in map {
            let toml_val = json_value_to_toml(val)?;
            table.insert(key.clone(), toml_val);
        }
    }
    Ok(table)
}

/// Recursively convert a serde_json::Value to a toml::Value.
fn json_value_to_toml(json: &serde_json::Value) -> Result<toml::Value> {
    match json {
        serde_json::Value::String(s) => Ok(toml::Value::String(s.clone())),
        serde_json::Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml::Value::Float(f))
            } else {
                Err(crate::error::AidotError::InvalidInput(
                    "Unsupported number type in MCP JSON".to_string(),
                ))
            }
        }
        serde_json::Value::Array(arr) => {
            let toml_arr: Result<Vec<toml::Value>> = arr.iter().map(json_value_to_toml).collect();
            Ok(toml::Value::Array(toml_arr?))
        }
        serde_json::Value::Object(_) => {
            let nested = json_value_to_toml_table(json)?;
            Ok(toml::Value::Table(nested))
        }
        serde_json::Value::Null => Ok(toml::Value::String(String::new())),
    }
}

impl ToolAdapter for CodexAdapter {
    fn name(&self) -> &str {
        "Codex CLI"
    }

    fn detect(&self) -> bool {
        self.codex_dir().exists() || is_command_available("codex")
    }

    fn scan(&self, preset_files: &PresetFiles, _target_dir: &Path) -> ScanResult {
        let mut result = ScanResult::new();
        let codex_dir = self.codex_dir();

        // rules + memory → .codex/AGENTS.md (merged)
        if !preset_files.rules.is_empty() || !preset_files.memory.is_empty() {
            result.add_change(
                ".codex/AGENTS.md".to_string(),
                "rules".to_string(),
                self.agents_md().exists(),
            );
        }

        // commands → .codex/skills/<name>/SKILL.md
        self.scan_skill_files(&preset_files.commands, "commands", &mut result);

        // skills → .codex/skills/<name>/SKILL.md
        self.scan_skill_files(&preset_files.skills, "skills", &mut result);

        // agents file: .codex/agents/<name>.md (1:1)
        scan_one_to_one(
            &preset_files.agents,
            "agents",
            &self.codex_agents_dir(),
            ".codex/agents",
            &mut result,
            None,
            None,
        );

        // mcp + agents config.toml
        let config_toml = &codex_dir.join("config.toml");
        let has_config_data = !preset_files.mcp.is_empty() || !preset_files.agents.is_empty();
        if has_config_data {
            scan_merged_section(
                // Use mcp as representative; agents is treated together
                if preset_files.mcp.is_empty() {
                    &preset_files.agents
                } else {
                    &preset_files.mcp
                },
                ".codex/config.toml",
                "mcp",
                config_toml,
                &mut result,
            );
        }

        result
    }

    fn apply(
        &self,
        preset_files: &PresetFiles,
        _target_dir: &Path,
        conflict_mode: &mut ConflictMode,
    ) -> Result<ApplyResult> {
        ensure_dir(&self.codex_dir())?;

        let mut result = ApplyResult::new();

        // Merged sections first (may trigger interactive prompts)
        self.apply_agents_md(
            &preset_files.rules,
            &preset_files.memory,
            &mut result,
            conflict_mode,
        )?;
        self.apply_config_toml(
            &preset_files.mcp,
            &preset_files.agents,
            &mut result,
            conflict_mode,
        )?;

        // commands → .codex/skills/<name>/SKILL.md
        self.apply_skill_files(
            &preset_files.commands,
            "commands",
            &mut result,
            conflict_mode,
        )?;

        // skills → .codex/skills/<name>/SKILL.md
        self.apply_skill_files(&preset_files.skills, "skills", &mut result, conflict_mode)?;

        // agents files: .codex/agents/<name>.md
        apply_one_to_one(
            &preset_files.agents,
            "agents",
            &self.codex_agents_dir(),
            ".codex/agents",
            &mut result,
            conflict_mode,
            None,
            None,
        )?;

        // settings: warn and skip
        if !preset_files.settings.is_empty() {
            use colored::Colorize;
            eprintln!(
                "  {} Codex CLI does not support 'settings' preset — skipped.",
                "Warning:".yellow()
            );
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_adapter() -> (TempDir, CodexAdapter) {
        let temp_dir = TempDir::new().unwrap();
        let adapter = CodexAdapter::new(temp_dir.path());
        (temp_dir, adapter)
    }

    #[test]
    fn test_adapter_name() {
        let (_dir, adapter) = create_adapter();
        assert_eq!(adapter.name(), "Codex CLI");
    }

    #[test]
    fn test_detect_no_codex() {
        let (temp_dir, _adapter) = create_adapter();
        let codex_dir = temp_dir.path().join(".codex");
        assert!(!codex_dir.exists());
    }

    #[test]
    fn test_detect_with_codex_dir() {
        let (temp_dir, adapter) = create_adapter();
        fs::create_dir_all(temp_dir.path().join(".codex")).unwrap();
        assert!(adapter.detect());
    }

    #[test]
    fn test_apply_rules_to_agents_md() {
        let (temp_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            rules: vec![PresetFile {
                relative_path: "rules/code-style.md".to_string(),
                content: "# Code Style".to_string(),
            }],
            ..Default::default()
        };

        let result = adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        assert_eq!(result.created.len(), 1);
        assert!(result.created[0].contains("AGENTS.md"));

        // Written to .codex/AGENTS.md, NOT .codex/rules/
        let agents_md = temp_dir.path().join(".codex/AGENTS.md");
        assert!(agents_md.exists());
        assert_eq!(fs::read_to_string(agents_md).unwrap(), "# Code Style");
    }

    #[test]
    fn test_apply_memory_to_agents_md() {
        let (temp_dir, adapter) = create_adapter();

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

        assert!(result.created.iter().any(|f| f.contains("AGENTS.md")));

        let agents_md = temp_dir.path().join(".codex/AGENTS.md");
        assert!(agents_md.exists());
        assert_eq!(fs::read_to_string(agents_md).unwrap(), "# Project Context");
    }

    #[test]
    fn test_apply_rules_and_memory_merged() {
        let (temp_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            rules: vec![PresetFile {
                relative_path: "rules/style.md".to_string(),
                content: "# Style Rules".to_string(),
            }],
            memory: vec![PresetFile {
                relative_path: "memory/context.md".to_string(),
                content: "# Context".to_string(),
            }],
            ..Default::default()
        };

        adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        let content = fs::read_to_string(temp_dir.path().join(".codex/AGENTS.md")).unwrap();
        // rules first, then memory, separated by ---
        assert!(content.contains("# Style Rules"));
        assert!(content.contains("---"));
        assert!(content.contains("# Context"));
    }

    #[test]
    fn test_apply_skills_creates_subdirectory() {
        let (temp_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            skills: vec![PresetFile {
                relative_path: "skills/code-review.md".to_string(),
                content: "---\nname: code-review\ndescription: Review code\n---\n# Code Review"
                    .to_string(),
            }],
            ..Default::default()
        };

        let result = adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        assert!(result.created.iter().any(|f| f.contains("SKILL.md")));

        let skill_file = temp_dir.path().join(".codex/skills/code-review/SKILL.md");
        assert!(skill_file.exists());
    }

    #[test]
    fn test_apply_skills_already_in_subdirectory() {
        let (temp_dir, adapter) = create_adapter();

        // Preset already has subdirectory structure: skills/<name>/SKILL.md
        let preset_files = PresetFiles {
            skills: vec![PresetFile {
                relative_path: "skills/cpp-coding-convention/SKILL.md".to_string(),
                content: "---\nname: cpp-coding-convention\ndescription: C++ conventions\n---\n# C++ Coding Convention".to_string(),
            }],
            ..Default::default()
        };

        adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        // Should be .codex/skills/cpp-coding-convention/SKILL.md, NOT .../SKILL/SKILL.md
        let correct = temp_dir
            .path()
            .join(".codex/skills/cpp-coding-convention/SKILL.md");
        let wrong = temp_dir
            .path()
            .join(".codex/skills/cpp-coding-convention/SKILL/SKILL.md");
        assert!(correct.exists(), "expected {:?} to exist", correct);
        assert!(!wrong.exists(), "unexpected {:?} should not exist", wrong);
    }

    #[test]
    fn test_apply_commands_become_skills() {
        let (temp_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            commands: vec![PresetFile {
                relative_path: "commands/build.md".to_string(),
                content: "# Build\n\nRun the build.".to_string(),
            }],
            ..Default::default()
        };

        adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        // commands are converted to skills
        let skill_file = temp_dir.path().join(".codex/skills/build/SKILL.md");
        assert!(skill_file.exists());
    }

    #[test]
    fn test_ensure_frontmatter_existing() {
        let content = "---\nname: test\ndescription: Test skill\n---\n# Content";
        let result = CodexAdapter::ensure_frontmatter("test", "skills", content);
        assert_eq!(result, content);
    }

    #[test]
    fn test_ensure_frontmatter_generates_from_h1() {
        let content = "# My Skill\n\nDoes something useful.";
        let result = CodexAdapter::ensure_frontmatter("my-skill", "skills", content);
        assert!(result.starts_with("---\n"));
        assert!(result.contains("name: my-skill"));
        assert!(result.contains("description: My Skill"));
        assert!(result.contains("# My Skill"));
    }

    #[test]
    fn test_ensure_frontmatter_fallback_to_stem() {
        let content = "No heading here, just text.";
        let result = CodexAdapter::ensure_frontmatter("my-skill", "skills", content);
        assert!(result.contains("name: my-skill"));
        assert!(result.contains("description: my-skill"));
    }

    #[test]
    fn test_ensure_frontmatter_commands_description() {
        let content = "# Build\n\nRun the build process.";
        let result = CodexAdapter::ensure_frontmatter("build", "commands", content);
        assert!(result.contains("name: build"));
        assert!(
            result.contains("description: Explicit command file for build."),
            "commands section should use explicit command description"
        );
    }

    #[test]
    fn test_ensure_frontmatter_injects_name_when_missing() {
        // Front matter exists with `paths:` but no `name` field
        let content =
            "---\npaths:\n  - \"**/*.{cpp,h,hpp}\"\n---\n\n# Doxygen Style Guide\n\nContent here.";
        let result = CodexAdapter::ensure_frontmatter("doxygen_style_guide", "skills", content);
        assert!(
            result.contains("name: doxygen_style_guide"),
            "should inject name"
        );
        assert!(
            result.contains("description: Doxygen Style Guide"),
            "should inject description from H1"
        );
        assert!(
            result.contains("paths:"),
            "should preserve existing paths field"
        );
        assert!(
            result.contains("**/*.{cpp,h,hpp}"),
            "should preserve existing paths value"
        );
    }

    #[test]
    fn test_ensure_frontmatter_keeps_existing_name() {
        // Front matter has `name` already — should be unchanged
        let content = "---\nname: my-custom-name\npaths:\n  - \"*.rs\"\n---\n# Title";
        let result = CodexAdapter::ensure_frontmatter("ignored-stem", "skills", content);
        assert_eq!(
            result, content,
            "should not modify content when name exists"
        );
    }

    #[test]
    fn test_apply_mcp_creates_toml() {
        let (temp_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            mcp: vec![PresetFile {
                relative_path: "mcp/context7.json".to_string(),
                content: r#"{"command":"npx","args":["-y","@upstash/context7-mcp"]}"#.to_string(),
            }],
            ..Default::default()
        };

        adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        let config_path = temp_dir.path().join(".codex/config.toml");
        assert!(config_path.exists());

        let toml_content = fs::read_to_string(&config_path).unwrap();
        assert!(toml_content.contains("[mcp_servers.context7]"));
        assert!(toml_content.contains("command = \"npx\""));
    }

    #[test]
    fn test_apply_mcp_with_env() {
        let (temp_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            mcp: vec![PresetFile {
                relative_path: "mcp/myserver.json".to_string(),
                content: r#"{"command":"node","args":["server.js"],"env":{"API_KEY":"abc"}}"#
                    .to_string(),
            }],
            ..Default::default()
        };

        adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        let toml_content = fs::read_to_string(temp_dir.path().join(".codex/config.toml")).unwrap();
        assert!(toml_content.contains("[mcp_servers.myserver.env]"));
        assert!(toml_content.contains("API_KEY = \"abc\""));
    }

    #[test]
    fn test_apply_agents_creates_file_and_config() {
        let (temp_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            agents: vec![PresetFile {
                relative_path: "agents/code-reviewer.md".to_string(),
                content: "# Code Reviewer\n\nReview code for quality.".to_string(),
            }],
            ..Default::default()
        };

        adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        // File copied to .codex/agents/
        let agent_file = temp_dir.path().join(".codex/agents/code-reviewer.md");
        assert!(agent_file.exists());

        // config.toml has [agents.code-reviewer]
        let config_path = temp_dir.path().join(".codex/config.toml");
        assert!(config_path.exists());
        let toml_content = fs::read_to_string(&config_path).unwrap();
        assert!(toml_content.contains("[agents.code-reviewer]"));
        assert!(toml_content.contains("role = \"Code Reviewer\""));
    }

    #[test]
    fn test_apply_mcp_and_agents_single_toml_write() {
        let (temp_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            mcp: vec![PresetFile {
                relative_path: "mcp/ctx.json".to_string(),
                content: r#"{"command":"npx","args":[]}"#.to_string(),
            }],
            agents: vec![PresetFile {
                relative_path: "agents/helper.md".to_string(),
                content: "# Helper Agent".to_string(),
            }],
            ..Default::default()
        };

        adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        let config_path = temp_dir.path().join(".codex/config.toml");
        let toml_content = fs::read_to_string(&config_path).unwrap();

        // Both sections must exist in the same file
        assert!(toml_content.contains("[mcp_servers.ctx]"));
        assert!(toml_content.contains("[agents.helper]"));
    }

    #[test]
    fn test_apply_mcp_merges_existing_toml() {
        let (temp_dir, adapter) = create_adapter();

        // Pre-existing config.toml with a model setting
        fs::create_dir_all(temp_dir.path().join(".codex")).unwrap();
        fs::write(
            temp_dir.path().join(".codex/config.toml"),
            "model = \"o3\"\n",
        )
        .unwrap();

        let preset_files = PresetFiles {
            mcp: vec![PresetFile {
                relative_path: "mcp/ctx.json".to_string(),
                content: r#"{"command":"npx","args":[]}"#.to_string(),
            }],
            ..Default::default()
        };

        adapter
            .apply(&preset_files, temp_dir.path(), &mut ConflictMode::Force)
            .unwrap();

        let toml_content = fs::read_to_string(temp_dir.path().join(".codex/config.toml")).unwrap();

        // Existing key should be preserved
        assert!(toml_content.contains("model = \"o3\""));
        // New section should be added
        assert!(toml_content.contains("[mcp_servers.ctx]"));
    }

    #[test]
    fn test_scan_rules_and_memory() {
        let (_dir, adapter) = create_adapter();

        let preset_files = PresetFiles {
            rules: vec![PresetFile {
                relative_path: "rules/style.md".to_string(),
                content: "# Style".to_string(),
            }],
            memory: vec![PresetFile {
                relative_path: "memory/ctx.md".to_string(),
                content: "# Context".to_string(),
            }],
            ..Default::default()
        };

        let scan = adapter.scan(&preset_files, Path::new("."));
        assert!(scan.has_changes());
        assert!(!scan.has_conflicts());
        // Both rules and memory go to .codex/AGENTS.md (single entry)
        assert!(scan.creates().iter().any(|c| c.path == ".codex/AGENTS.md"));
        assert_eq!(
            scan.creates()
                .iter()
                .filter(|c| c.path == ".codex/AGENTS.md")
                .count(),
            1,
            "rules + memory should produce a single .codex/AGENTS.md entry"
        );
    }

    #[test]
    fn test_extract_h1_found() {
        assert_eq!(
            extract_h1("# My Title\n\nBody"),
            Some("My Title".to_string())
        );
    }

    #[test]
    fn test_extract_h1_not_found() {
        assert_eq!(extract_h1("No heading here."), None);
    }
}
