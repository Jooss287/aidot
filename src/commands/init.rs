use crate::error::{AidotError, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Extracted file info from existing LLM tool configurations
#[derive(Debug, Default)]
struct ExtractedFiles {
    rules: Vec<(String, String)>,    // (filename, content)
    memory: Vec<(String, String)>,   // (filename, content)
    commands: Vec<(String, String)>, // (filename, content)
    mcp: Vec<(String, String)>,      // (filename, content)
    hooks: Vec<(String, String)>,    // (filename, content)
    agents: Vec<(String, String)>,   // (filename, content)
    skills: Vec<(String, String)>,   // (filename, content)
    settings: Vec<(String, String)>, // (filename, content)
}

impl ExtractedFiles {
    fn is_empty(&self) -> bool {
        self.rules.is_empty()
            && self.memory.is_empty()
            && self.commands.is_empty()
            && self.mcp.is_empty()
            && self.hooks.is_empty()
            && self.agents.is_empty()
            && self.skills.is_empty()
            && self.settings.is_empty()
    }
}

/// Initialize a new preset repository
pub fn init_preset(
    path: Option<String>,
    from_existing: bool,
    _interactive: bool,
    force: bool,
) -> Result<()> {
    let target_dir = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        std::env::current_dir()?
    };

    // Check if .aidot-config.toml already exists
    let config_file = target_dir.join(".aidot-config.toml");
    if config_file.exists() && !force {
        return Err(AidotError::PresetAlreadyExists(target_dir));
    }

    if from_existing {
        init_from_existing(&target_dir)?;
    } else {
        init_empty_preset(&target_dir)?;
    }

    println!(
        "{} {}",
        "✓ Preset repository initialized at".green().bold(),
        target_dir.display().to_string().white()
    );
    Ok(())
}

/// Initialize an empty preset repository
fn init_empty_preset(path: &Path) -> Result<()> {
    println!(
        "{}\n",
        "Initializing empty aidot preset repository...".cyan()
    );

    // Create directory structure
    let directories = vec![
        "rules", "memory", "commands", "mcp", "hooks", "agents", "skills", "settings",
    ];

    for dir in &directories {
        let dir_path = path.join(dir);
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)?;
            println!("  {} {} {}/", "✓".green(), "Created".green(), dir.white());
        }
    }

    // Create .aidot-config.toml with comments
    let preset_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("llm-preset");
    let config_content = create_config_template(preset_name);
    fs::write(path.join(".aidot-config.toml"), config_content)?;
    println!(
        "  {} {} {}",
        "✓".green(),
        "Created".green(),
        ".aidot-config.toml".white()
    );

    // Create README.md
    create_readme(path, preset_name)?;

    println!("\n{}", "Preset repository initialized!".green().bold());
    println!("\n{}:", "Next steps".cyan().bold());
    println!(
        "  {} Add your configuration files to {}, {}, {}, etc.",
        "1.".white(),
        "rules/".cyan(),
        "memory/".cyan(),
        "commands/".cyan()
    );
    println!(
        "  {} Customize {}",
        "2.".white(),
        ".aidot-config.toml".cyan()
    );
    println!(
        "  {} {}",
        "3.".white(),
        "git init && git add . && git commit -m 'Initial preset'".dimmed()
    );
    println!("  {} Push to your Git repository", "4.".white());
    println!(
        "  {} Use with: {}",
        "5.".white(),
        "aidot repo add <name> <url>".cyan()
    );

    Ok(())
}

/// Initialize preset from existing LLM configurations
fn init_from_existing(path: &Path) -> Result<()> {
    println!(
        "{}\n",
        "Extracting preset from existing LLM configurations...".cyan()
    );

    let mut extracted = ExtractedFiles::default();
    let mut sources_found: Vec<String> = Vec::new();

    // Scan for Claude Code configurations
    if let Some(count) = extract_claude_code(path, &mut extracted)? {
        sources_found.push(format!("Claude Code ({} files)", count));
    }

    // Scan for Cursor configurations
    if let Some(count) = extract_cursor(path, &mut extracted)? {
        sources_found.push(format!("Cursor ({} files)", count));
    }

    // Scan for GitHub Copilot configurations
    if let Some(count) = extract_copilot(path, &mut extracted)? {
        sources_found.push(format!("GitHub Copilot ({} files)", count));
    }

    if extracted.is_empty() {
        println!(
            "{} {}",
            "⚠".yellow(),
            "No existing LLM configurations found.".yellow()
        );
        println!(
            "  {}",
            "Looked for: .claude/, .cursor/, .cursorrules, .github/".dimmed()
        );
        println!(
            "\n  {} {}",
            "Tip:".cyan(),
            "Use 'aidot init' to create an empty preset instead.".white()
        );
        return Ok(());
    }

    // Print sources found
    println!("{}", "Found configurations from:".cyan());
    for source in &sources_found {
        println!("  {} {}", "•".cyan(), source.white());
    }
    println!();

    // Create directory structure
    let directories = vec![
        "rules", "memory", "commands", "mcp", "hooks", "agents", "skills", "settings",
    ];

    for dir in &directories {
        let dir_path = path.join(dir);
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)?;
        }
    }

    // Write extracted files
    let mut written_count = 0;

    written_count += write_extracted_files(path, "rules", &extracted.rules)?;
    written_count += write_extracted_files(path, "memory", &extracted.memory)?;
    written_count += write_extracted_files(path, "commands", &extracted.commands)?;
    written_count += write_extracted_files(path, "mcp", &extracted.mcp)?;
    written_count += write_extracted_files(path, "hooks", &extracted.hooks)?;
    written_count += write_extracted_files(path, "agents", &extracted.agents)?;
    written_count += write_extracted_files(path, "skills", &extracted.skills)?;
    written_count += write_extracted_files(path, "settings", &extracted.settings)?;

    // Create .aidot-config.toml with comments
    let preset_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("llm-preset");
    let config_content = create_config_template(preset_name);
    fs::write(path.join(".aidot-config.toml"), config_content)?;
    println!(
        "  {} {} {}",
        "✓".green(),
        "Created".green(),
        ".aidot-config.toml".white()
    );

    // Create README.md
    create_readme(path, preset_name)?;

    println!(
        "\n{} {} files extracted from existing configurations",
        "✓".green(),
        written_count.to_string().white().bold()
    );

    println!("\n{}:", "Next steps".cyan().bold());
    println!(
        "  {} Review and organize the extracted files in {}, {}, etc.",
        "1.".white(),
        "rules/".cyan(),
        "memory/".cyan()
    );
    println!(
        "  {} Remove any tool-specific content that shouldn't be shared",
        "2.".white()
    );
    println!(
        "  {} Customize {}",
        "3.".white(),
        ".aidot-config.toml".cyan()
    );
    println!(
        "  {} {}",
        "4.".white(),
        "git init && git add . && git commit -m 'Initial preset'".dimmed()
    );

    Ok(())
}

/// Write extracted files to a directory
fn write_extracted_files(
    base_path: &Path,
    dir_name: &str,
    files: &[(String, String)],
) -> Result<usize> {
    let dir_path = base_path.join(dir_name);
    let mut count = 0;

    for (filename, content) in files {
        let file_path = dir_path.join(filename);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(&file_path, content)?;
        println!(
            "  {} {} {}/{}",
            "✓".green(),
            "Extracted".green(),
            dir_name.cyan(),
            filename.white()
        );
        count += 1;
    }

    Ok(count)
}

/// Extract configurations from Claude Code (.claude/)
fn extract_claude_code(
    source_path: &Path,
    extracted: &mut ExtractedFiles,
) -> Result<Option<usize>> {
    let claude_dir = source_path.join(".claude");
    if !claude_dir.exists() {
        return Ok(None);
    }

    let mut count = 0;

    // .claude/rules/ → rules/
    let rules_dir = claude_dir.join("rules");
    if rules_dir.exists() {
        for entry in WalkDir::new(&rules_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    if let Ok(content) = fs::read_to_string(path) {
                        let relative = path.strip_prefix(&rules_dir).unwrap_or(path);
                        extracted
                            .rules
                            .push((relative.to_string_lossy().to_string(), content));
                        count += 1;
                    }
                }
            }
        }
    }

    // .claude/CLAUDE.md → memory/claude-memory.md
    let claude_md = claude_dir.join("CLAUDE.md");
    if claude_md.exists() {
        if let Ok(content) = fs::read_to_string(&claude_md) {
            extracted
                .memory
                .push(("claude-memory.md".to_string(), content));
            count += 1;
        }
    }

    // Root CLAUDE.md → memory/project-memory.md
    let root_claude_md = source_path.join("CLAUDE.md");
    if root_claude_md.exists() {
        if let Ok(content) = fs::read_to_string(&root_claude_md) {
            extracted
                .memory
                .push(("project-memory.md".to_string(), content));
            count += 1;
        }
    }

    // .claude/commands/ → commands/
    let commands_dir = claude_dir.join("commands");
    if commands_dir.exists() {
        for entry in WalkDir::new(&commands_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    if let Ok(content) = fs::read_to_string(path) {
                        let relative = path.strip_prefix(&commands_dir).unwrap_or(path);
                        extracted
                            .commands
                            .push((relative.to_string_lossy().to_string(), content));
                        count += 1;
                    }
                }
            }
        }
    }

    // .claude/settings.local.json → mcp/ and settings/
    let settings_file = claude_dir.join("settings.local.json");
    if settings_file.exists() {
        if let Ok(content) = fs::read_to_string(&settings_file) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Extract mcpServers section
                if let Some(mcp_servers) = json.get("mcpServers") {
                    if let Some(obj) = mcp_servers.as_object() {
                        for (name, config) in obj {
                            let mcp_content =
                                serde_json::to_string_pretty(config).unwrap_or_default();
                            extracted.mcp.push((format!("{}.json", name), mcp_content));
                            count += 1;
                        }
                    }
                }

                // Extract other settings (exclude mcpServers)
                let mut settings_obj = json.clone();
                if let Some(obj) = settings_obj.as_object_mut() {
                    obj.remove("mcpServers");
                    if !obj.is_empty() {
                        let settings_content =
                            serde_json::to_string_pretty(&settings_obj).unwrap_or_default();
                        extracted
                            .settings
                            .push(("claude-settings.json".to_string(), settings_content));
                        count += 1;
                    }
                }
            }
        }
    }

    // .claude/hooks.json → hooks/
    let hooks_file = claude_dir.join("hooks.json");
    if hooks_file.exists() {
        if let Ok(content) = fs::read_to_string(&hooks_file) {
            extracted
                .hooks
                .push(("claude-hooks.json".to_string(), content));
            count += 1;
        }
    }

    // .claude/agents/ → agents/
    let agents_dir = claude_dir.join("agents");
    if agents_dir.exists() {
        for entry in WalkDir::new(&agents_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Ok(content) = fs::read_to_string(path) {
                let relative = path.strip_prefix(&agents_dir).unwrap_or(path);
                extracted
                    .agents
                    .push((relative.to_string_lossy().to_string(), content));
                count += 1;
            }
        }
    }

    // .claude/skills/ → skills/
    let skills_dir = claude_dir.join("skills");
    if skills_dir.exists() {
        for entry in WalkDir::new(&skills_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Ok(content) = fs::read_to_string(path) {
                let relative = path.strip_prefix(&skills_dir).unwrap_or(path);
                extracted
                    .skills
                    .push((relative.to_string_lossy().to_string(), content));
                count += 1;
            }
        }
    }

    if count > 0 {
        Ok(Some(count))
    } else {
        Ok(None)
    }
}

/// Extract configurations from Cursor (.cursorrules, .cursor/)
fn extract_cursor(source_path: &Path, extracted: &mut ExtractedFiles) -> Result<Option<usize>> {
    let mut count = 0;

    // .cursorrules → rules/cursorrules.md
    let cursorrules = source_path.join(".cursorrules");
    if cursorrules.exists() {
        if let Ok(content) = fs::read_to_string(&cursorrules) {
            extracted
                .rules
                .push(("cursorrules.md".to_string(), content));
            count += 1;
        }
    }

    let cursor_dir = source_path.join(".cursor");
    if !cursor_dir.exists() && count == 0 {
        return Ok(None);
    }

    if cursor_dir.exists() {
        // .cursor/rules/ → rules/
        let rules_dir = cursor_dir.join("rules");
        if rules_dir.exists() {
            for entry in WalkDir::new(&rules_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "md" || ext == "mdc" {
                        if let Ok(content) = fs::read_to_string(path) {
                            let relative = path.strip_prefix(&rules_dir).unwrap_or(path);
                            let filename = relative.to_string_lossy().to_string();
                            // Convert .mdc to .md
                            let filename = if filename.ends_with(".mdc") {
                                filename.replace(".mdc", ".md")
                            } else {
                                filename
                            };
                            // Prefix with cursor- to avoid conflicts
                            let prefixed = format!("cursor-{}", filename);
                            extracted.rules.push((prefixed, content));
                            count += 1;
                        }
                    }
                }
            }
        }

        // .cursor/commands/ → commands/
        let commands_dir = cursor_dir.join("commands");
        if commands_dir.exists() {
            for entry in WalkDir::new(&commands_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if let Ok(content) = fs::read_to_string(path) {
                    let relative = path.strip_prefix(&commands_dir).unwrap_or(path);
                    extracted
                        .commands
                        .push((relative.to_string_lossy().to_string(), content));
                    count += 1;
                }
            }
        }

        // .cursor/mcp.json → mcp/
        let mcp_file = cursor_dir.join("mcp.json");
        if mcp_file.exists() {
            if let Ok(content) = fs::read_to_string(&mcp_file) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(mcp_servers) = json.get("mcpServers") {
                        if let Some(obj) = mcp_servers.as_object() {
                            for (name, config) in obj {
                                let mcp_content =
                                    serde_json::to_string_pretty(config).unwrap_or_default();
                                // Prefix with cursor- to avoid conflicts
                                extracted
                                    .mcp
                                    .push((format!("cursor-{}.json", name), mcp_content));
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        // .cursor/hooks.json → hooks/
        let hooks_file = cursor_dir.join("hooks.json");
        if hooks_file.exists() {
            if let Ok(content) = fs::read_to_string(&hooks_file) {
                extracted
                    .hooks
                    .push(("cursor-hooks.json".to_string(), content));
                count += 1;
            }
        }

        // .cursor/agents/ → agents/
        let agents_dir = cursor_dir.join("agents");
        if agents_dir.exists() {
            for entry in WalkDir::new(&agents_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if let Ok(content) = fs::read_to_string(path) {
                    let relative = path.strip_prefix(&agents_dir).unwrap_or(path);
                    extracted
                        .agents
                        .push((relative.to_string_lossy().to_string(), content));
                    count += 1;
                }
            }
        }

        // .cursor/skills/ → skills/
        let skills_dir = cursor_dir.join("skills");
        if skills_dir.exists() {
            for entry in WalkDir::new(&skills_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if let Ok(content) = fs::read_to_string(path) {
                    let relative = path.strip_prefix(&skills_dir).unwrap_or(path);
                    extracted
                        .skills
                        .push((relative.to_string_lossy().to_string(), content));
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        Ok(Some(count))
    } else {
        Ok(None)
    }
}

/// Extract configurations from GitHub Copilot (.github/)
fn extract_copilot(source_path: &Path, extracted: &mut ExtractedFiles) -> Result<Option<usize>> {
    let github_dir = source_path.join(".github");
    if !github_dir.exists() {
        return Ok(None);
    }

    let mut count = 0;

    // .github/copilot-instructions.md → rules/copilot-instructions.md
    let instructions = github_dir.join("copilot-instructions.md");
    if instructions.exists() {
        if let Ok(content) = fs::read_to_string(&instructions) {
            extracted
                .rules
                .push(("copilot-instructions.md".to_string(), content));
            count += 1;
        }
    }

    // .github/instructions/*.instructions.md → rules/
    let instructions_dir = github_dir.join("instructions");
    if instructions_dir.exists() {
        for entry in WalkDir::new(&instructions_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".instructions.md") {
                    if let Ok(content) = fs::read_to_string(path) {
                        // Convert .instructions.md to .md
                        let filename = name.replace(".instructions.md", ".md");
                        extracted
                            .rules
                            .push((format!("copilot-{}", filename), content));
                        count += 1;
                    }
                }
            }
        }
    }

    // .github/prompts/*.prompt.md → commands/
    let prompts_dir = github_dir.join("prompts");
    if prompts_dir.exists() {
        for entry in WalkDir::new(&prompts_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".prompt.md") {
                    if let Ok(content) = fs::read_to_string(path) {
                        // Convert .prompt.md to .md
                        let filename = name.replace(".prompt.md", ".md");
                        extracted.commands.push((filename, content));
                        count += 1;
                    }
                }
            }
        }
    }

    // .vscode/mcp.json → mcp/
    let vscode_dir = source_path.join(".vscode");
    let mcp_file = vscode_dir.join("mcp.json");
    if mcp_file.exists() {
        if let Ok(content) = fs::read_to_string(&mcp_file) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Check both "mcpServers" and "servers" keys
                let servers = json.get("mcpServers").or_else(|| json.get("servers"));

                if let Some(servers_obj) = servers.and_then(|s| s.as_object()) {
                    for (name, config) in servers_obj {
                        let mcp_content = serde_json::to_string_pretty(config).unwrap_or_default();
                        extracted
                            .mcp
                            .push((format!("vscode-{}.json", name), mcp_content));
                        count += 1;
                    }
                }
            }
        }
    }

    // .github/agents/*.agent.md → agents/
    let agents_dir = github_dir.join("agents");
    if agents_dir.exists() {
        for entry in WalkDir::new(&agents_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".agent.md") {
                    if let Ok(content) = fs::read_to_string(path) {
                        // Convert .agent.md to .md
                        let filename = name.replace(".agent.md", ".md");
                        extracted.agents.push((filename, content));
                        count += 1;
                    }
                }
            }
        }
    }

    // .github/skills/ → skills/
    let skills_dir = github_dir.join("skills");
    if skills_dir.exists() {
        for entry in WalkDir::new(&skills_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Ok(content) = fs::read_to_string(path) {
                let relative = path.strip_prefix(&skills_dir).unwrap_or(path);
                extracted
                    .skills
                    .push((relative.to_string_lossy().to_string(), content));
                count += 1;
            }
        }
    }

    if count > 0 {
        Ok(Some(count))
    } else {
        Ok(None)
    }
}

/// Create .aidot-config.toml template with comments
fn create_config_template(preset_name: &str) -> String {
    format!(
        r#"[metadata]
name = "{}"
version = "1.0.0"
description = "LLM configuration preset"

# Rules: LLM behavioral rules and coding guidelines
# You can specify individual files or use a directory
[rules]
directory = "rules/"
# files = ["rules/code-style.md", "rules/testing.md"]

# Memory: Project context and documentation
[memory]
directory = "memory/"

# Commands: Custom slash commands
[commands]
directory = "commands/"

# MCP: Model Context Protocol server configurations
[mcp]
directory = "mcp/"

# Hooks: Event-based automation hooks
[hooks]
directory = "hooks/"

# Agents: AI agent definitions
[agents]
directory = "agents/"

# Skills: Reusable agent utilities
[skills]
directory = "skills/"

# Settings: Tool-specific settings
[settings]
directory = "settings/"
"#,
        preset_name
    )
}

/// Create README.md for the preset repository
fn create_readme(path: &Path, preset_name: &str) -> Result<()> {
    let readme = format!(
        r#"# {} - LLM Configuration Preset

This repository contains LLM tool configurations managed by [aidot](https://github.com/yourorg/aidot).

## Structure

- `rules/` - LLM behavioral rules and coding guidelines
- `memory/` - Project memory (architecture, workflows, standards)
- `commands/` - Custom slash commands
- `mcp/` - MCP (Model Context Protocol) server configurations
- `hooks/` - Event-based automation hooks
- `agents/` - AI agent definitions
- `skills/` - Reusable agent utilities
- `settings/` - Tool-specific settings

## Example Files

### rules/
Add markdown files with coding guidelines and LLM behavioral rules:
- `rules/code-style.md` - Code style guidelines
- `rules/testing.md` - Testing conventions
- `rules/security.md` - Security requirements

### memory/
Add project context and documentation:
- `memory/architecture.md` - Project architecture overview
- `memory/workflows.md` - Development workflows
- `memory/coding-standards.md` - Team coding standards

### commands/
Add custom slash commands (for Claude Code, Cursor, etc.):
- `commands/analyze.md` - Code analysis command
- `commands/test.md` - Test generation command

### mcp/
Add MCP server configurations (JSON format):
- `mcp/filesystem.json` - Filesystem access
- `mcp/github.json` - GitHub integration

### hooks/
Add event-based automation hooks (JSON format):
- `hooks/pre-tool-use.json` - Run before tool execution
- `hooks/post-tool-use.json` - Run after tool execution

### agents/
Add AI agent definitions (markdown format):
- `agents/code-reviewer.md` - Code review agent
- `agents/test-generator.md` - Test generation agent

### skills/
Add reusable agent utilities (TypeScript/JavaScript):
- `skills/api-client.ts` - API client utility
- `skills/data-analyzer.ts` - Data analysis helpers

### settings/
Add tool-specific settings (JSON format):
- `settings/preferences.json` - General preferences

## Usage

### Install aidot

```bash
# Installation instructions for aidot
```

### Add this preset

```bash
aidot repo add {} <repository-url>
```

### Apply to your project

```bash
cd your-project
aidot pull {}
```

## Customization

Edit the files in this repository to customize the configuration for your team or project.

## Supported Tools

This preset is compatible with:
- Claude Code
- Cursor
- Aider
- GitHub Copilot
- Continue

aidot automatically converts these configurations to the appropriate format for each tool.
"#,
        preset_name, preset_name, preset_name
    );

    fs::write(path.join("README.md"), readme)?;
    println!(
        "  {} {} {}",
        "✓".green(),
        "Created".green(),
        "README.md".white()
    );

    Ok(())
}
