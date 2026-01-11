use crate::error::{AidotError, Result};
use crate::template::TemplateConfig;
use std::fs;
use std::path::{Path, PathBuf};

/// Initialize a new template repository
pub fn init_template(
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
        return Err(AidotError::TemplateAlreadyExists(target_dir));
    }

    if from_existing {
        init_from_existing(&target_dir)?;
    } else {
        init_empty_template(&target_dir)?;
    }

    println!("✓ Template repository initialized at {}", target_dir.display());
    Ok(())
}

/// Initialize an empty template repository
fn init_empty_template(path: &Path) -> Result<()> {
    println!("Initializing empty aidot template repository...\n");

    // Create directory structure
    let directories = vec![
        "rules",
        "memory",
        "commands",
        "mcp",
        "hooks",
        "agents",
        "skills",
        "settings",
    ];

    for dir in &directories {
        let dir_path = path.join(dir);
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)?;
            println!("  ✓ Created {}/", dir);
        }
    }

    // Create .aidot-config.toml
    let template_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("llm-template");
    let config = TemplateConfig::default_template(template_name);
    config.save(path)?;
    println!("  ✓ Created .aidot-config.toml");

    // Create README.md
    create_readme(path, template_name)?;

    println!("\nTemplate repository initialized!");
    println!("\nNext steps:");
    println!("  1. Add your configuration files to rules/, memory/, commands/, etc.");
    println!("  2. Customize .aidot-config.toml");
    println!("  3. git init && git add . && git commit -m 'Initial template'");
    println!("  4. Push to your Git repository");
    println!("  5. Use with: aidot repo add <name> <url>");

    Ok(())
}

/// Initialize template from existing LLM configurations
fn init_from_existing(_path: &Path) -> Result<()> {
    // TODO: Implement in Phase 6
    Err(AidotError::InvalidTemplate(
        "--from-existing is not yet implemented".to_string(),
    ))
}

/// Create README.md for the template repository
fn create_readme(path: &Path, template_name: &str) -> Result<()> {
    let readme = format!(
        r#"# {} - LLM Configuration Template

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

### Add this template

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

This template is compatible with:
- Claude Code
- Cursor
- Aider
- GitHub Copilot
- Continue

aidot automatically converts these configurations to the appropriate format for each tool.
"#,
        template_name, template_name, template_name
    );

    fs::write(path.join("README.md"), readme)?;
    println!("  ✓ Created README.md");

    Ok(())
}
