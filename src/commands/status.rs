use crate::adapters::detector::get_detected_tool_names;
use crate::config::Config;
use crate::error::Result;
use colored::Colorize;
use std::env;
use std::fs;
use std::path::Path;

/// Show current configuration status
pub fn show_status() -> Result<()> {
    let current_dir = env::current_dir()?;

    println!(
        "{} {}\n",
        "Project:".cyan().bold(),
        current_dir.display().to_string().white()
    );

    // Show detected tools
    println!("{}", "═══ Detected Tools ═══".cyan().bold());
    let tools = get_detected_tool_names(&current_dir);
    let detected_tools: Vec<_> = tools.iter().filter(|t| t.detected).collect();

    if detected_tools.is_empty() {
        println!("  {}", "No LLM tools detected".yellow());
    } else {
        for tool in &detected_tools {
            println!("  {} {}", "✓".green(), tool.name.white());
        }
    }
    println!();

    // Show configuration files for each tool
    println!("{}", "═══ Configuration Files ═══".cyan().bold());

    // Claude Code
    let claude_dir = current_dir.join(".claude");
    if claude_dir.exists() {
        println!("\n  {}:", "Claude Code".white().bold());
        show_dir_contents(&claude_dir, "    ")?;
    }

    // Cursor
    let cursor_dir = current_dir.join(".cursor");
    let cursorrules = current_dir.join(".cursorrules");
    if cursor_dir.exists() || cursorrules.exists() {
        println!("\n  {}:", "Cursor".white().bold());
        if cursorrules.exists() {
            let size = fs::metadata(&cursorrules)?.len();
            println!(
                "    {} {} {}",
                "•".cyan(),
                ".cursorrules".white(),
                format!("({} bytes)", size).dimmed()
            );
        }
        if cursor_dir.exists() {
            show_dir_contents(&cursor_dir, "    ")?;
        }
    }

    // GitHub Copilot
    let github_dir = current_dir.join(".github");
    let vscode_dir = current_dir.join(".vscode");
    let copilot_instructions = github_dir.join("copilot-instructions.md");

    if copilot_instructions.exists() || github_dir.join("prompts").exists() {
        println!("\n  {}:", "GitHub Copilot".white().bold());
        if copilot_instructions.exists() {
            let size = fs::metadata(&copilot_instructions)?.len();
            println!(
                "    {} {} {}",
                "•".cyan(),
                ".github/copilot-instructions.md".white(),
                format!("({} bytes)", size).dimmed()
            );
        }
        if github_dir.join("prompts").exists() {
            show_dir_contents(&github_dir.join("prompts"), "    ")?;
        }
        if github_dir.join("agents").exists() {
            show_dir_contents(&github_dir.join("agents"), "    ")?;
        }
    }

    // VS Code MCP
    if vscode_dir.join("mcp.json").exists() {
        println!("\n  {}:", "VS Code (MCP)".white().bold());
        let size = fs::metadata(vscode_dir.join("mcp.json"))?.len();
        println!(
            "    {} {} {}",
            "•".cyan(),
            ".vscode/mcp.json".white(),
            format!("({} bytes)", size).dimmed()
        );
    }

    println!();

    // Show registered repositories
    println!("{}", "═══ Registered Repositories ═══".cyan().bold());
    let config = Config::load()?;

    if config.repositories.is_empty() {
        println!("  {}", "No repositories registered".dimmed());
        println!(
            "  {}",
            "Use 'aidot repo add <name> <url>' to register a preset repository".dimmed()
        );
    } else {
        for repo in &config.repositories {
            let mut flags = Vec::new();
            if repo.source_type == crate::config::SourceType::Local {
                flags.push("local".yellow());
            }
            if repo.default {
                flags.push("default".green());
            }

            let flags_str = if flags.is_empty() {
                String::new()
            } else {
                format!(
                    " [{}]",
                    flags
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                        .join("] [")
                )
            };

            println!(
                "  {} {} {}{}",
                "•".cyan(),
                repo.name.white().bold(),
                repo.url.dimmed(),
                flags_str
            );
        }
    }

    println!();

    Ok(())
}

/// Show contents of a directory
fn show_dir_contents(dir: &Path, indent: &str) -> Result<()> {
    let dir_name = dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.display().to_string());

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            println!(
                "{}{} {}/",
                indent,
                "•".cyan(),
                format!("{}/{}", dir_name, name).white()
            );
            // Optionally show subdirectory contents (one level deep)
            if let Ok(subdir) = fs::read_dir(&path) {
                for subentry in subdir.take(5) {
                    if let Ok(subentry) = subentry {
                        let subname = subentry.file_name().to_string_lossy().to_string();
                        println!(
                            "{}  {} {}",
                            indent,
                            "·".dimmed(),
                            subname.dimmed()
                        );
                    }
                }
            }
        } else {
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            println!(
                "{}{} {}/{} {}",
                indent,
                "•".cyan(),
                dir_name.dimmed(),
                name.white(),
                format!("({} bytes)", size).dimmed()
            );
        }
    }

    Ok(())
}
