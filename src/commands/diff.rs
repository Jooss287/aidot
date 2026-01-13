use crate::adapters::detector::detect_tools;
use crate::adapters::traits::PresetFiles;
use crate::error::Result;
use crate::repository::resolve_repository_source;
use crate::preset::parser::parse_preset;
use colored::Colorize;
use std::fs;
use std::path::Path;

/// Show diff between preset and current configuration
pub fn show_diff(repo_source: String) -> Result<()> {
    let target_dir = std::env::current_dir()?;

    // Resolve repository source
    let preset_path = resolve_repository_source(&repo_source)?;

    // Parse preset
    let (_config, preset_files) = parse_preset(&preset_path)?;

    println!(
        "{} '{}'\n",
        "Comparing preset".cyan().bold(),
        repo_source.white()
    );

    // Detect tools
    let tools = detect_tools(&target_dir);

    if tools.is_empty() {
        println!(
            "{} {}",
            "⚠".yellow(),
            "No LLM tools detected in current directory.".yellow()
        );
        return Ok(());
    }

    let mut total_new = 0;
    let mut total_modified = 0;
    let mut total_unchanged = 0;

    for tool in &tools {
        println!("{}", format!("═══ {} ═══", tool.name()).cyan().bold());

        let diff_result = compute_diff(tool.name(), &preset_files, &target_dir)?;

        if diff_result.new_files.is_empty()
            && diff_result.modified_files.is_empty()
            && diff_result.unchanged_files.is_empty()
        {
            println!("  {} No preset files for this tool\n", "○".dimmed());
            continue;
        }

        // New files (would be created)
        if !diff_result.new_files.is_empty() {
            println!("  {} New files:", "+".green().bold());
            for file in &diff_result.new_files {
                println!("    {} {}", "+".green(), file.white());
            }
            total_new += diff_result.new_files.len();
        }

        // Modified files (content differs)
        if !diff_result.modified_files.is_empty() {
            println!("  {} Modified files:", "~".yellow().bold());
            for (file, diff_info) in &diff_result.modified_files {
                println!("    {} {}", "~".yellow(), file.white());
                if let Some(info) = diff_info {
                    println!("      {}", info.dimmed());
                }
            }
            total_modified += diff_result.modified_files.len();
        }

        // Unchanged files
        if !diff_result.unchanged_files.is_empty() {
            println!("  {} Unchanged files:", "=".dimmed());
            for file in &diff_result.unchanged_files {
                println!("    {} {}", "=".dimmed(), file.dimmed());
            }
            total_unchanged += diff_result.unchanged_files.len();
        }

        println!();
    }

    // Summary
    println!("{}", "═══ Summary ═══".cyan().bold());
    println!(
        "  {} {} new, {} {} modified, {} {} unchanged",
        total_new.to_string().green().bold(),
        "files".green(),
        total_modified.to_string().yellow().bold(),
        "files".yellow(),
        total_unchanged.to_string().dimmed(),
        "files".dimmed()
    );

    if total_new > 0 || total_modified > 0 {
        println!(
            "\n  {} Run {} to apply changes",
            "Tip:".cyan(),
            format!("aidot pull {}", repo_source).white().bold()
        );
    }

    Ok(())
}

/// Diff result for a single tool
struct DiffResult {
    new_files: Vec<String>,
    modified_files: Vec<(String, Option<String>)>, // (filename, diff_info)
    unchanged_files: Vec<String>,
}

/// Compute diff between preset and current tool configuration
fn compute_diff(tool_name: &str, preset: &PresetFiles, target_dir: &Path) -> Result<DiffResult> {
    let mut result = DiffResult {
        new_files: Vec::new(),
        modified_files: Vec::new(),
        unchanged_files: Vec::new(),
    };

    match tool_name {
        "Claude Code" => compute_claude_diff(preset, target_dir, &mut result)?,
        "Cursor" => compute_cursor_diff(preset, target_dir, &mut result)?,
        "GitHub Copilot" => compute_copilot_diff(preset, target_dir, &mut result)?,
        _ => {}
    }

    Ok(result)
}

/// Compute diff for Claude Code
fn compute_claude_diff(
    preset: &PresetFiles,
    target_dir: &Path,
    result: &mut DiffResult,
) -> Result<()> {
    let claude_dir = target_dir.join(".claude");

    // Rules: preset rules/*.md → .claude/rules/
    for file in &preset.rules {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let target_file = claude_dir.join("rules").join(&*filename);
        compare_file(&target_file, &file.content, &filename, result);
    }

    // Memory: preset memory/*.md → .claude/CLAUDE.md (merged)
    if !preset.memory.is_empty() {
        let target_file = claude_dir.join("CLAUDE.md");
        let preset_content: String = preset
            .memory
            .iter()
            .map(|f| f.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        compare_file(&target_file, &preset_content, "CLAUDE.md", result);
    }

    // Commands: preset commands/*.md → .claude/commands/
    for file in &preset.commands {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let target_file = claude_dir.join("commands").join(&*filename);
        compare_file(&target_file, &file.content, &format!("commands/{}", filename), result);
    }

    // MCP: preset mcp/*.json → .claude/settings.local.json (mcpServers section)
    if !preset.mcp.is_empty() {
        let target_file = claude_dir.join("settings.local.json");
        if target_file.exists() {
            // Check if mcpServers section exists and differs
            if let Ok(existing) = fs::read_to_string(&target_file) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&existing) {
                    if json.get("mcpServers").is_some() {
                        result.modified_files.push((
                            "settings.local.json (mcpServers)".to_string(),
                            Some(format!("{} MCP servers in preset", preset.mcp.len())),
                        ));
                    } else {
                        result.new_files.push("settings.local.json (mcpServers section)".to_string());
                    }
                }
            }
        } else {
            result.new_files.push("settings.local.json".to_string());
        }
    }

    // Hooks
    if !preset.hooks.is_empty() {
        let target_file = claude_dir.join("hooks.json");
        compare_file_exists(&target_file, "hooks.json", result);
    }

    // Agents
    for file in &preset.agents {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let target_file = claude_dir.join("agents").join(&*filename);
        compare_file(&target_file, &file.content, &format!("agents/{}", filename), result);
    }

    // Skills
    for file in &preset.skills {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let target_file = claude_dir.join("skills").join(&*filename);
        compare_file(&target_file, &file.content, &format!("skills/{}", filename), result);
    }

    Ok(())
}

/// Compute diff for Cursor
fn compute_cursor_diff(
    preset: &PresetFiles,
    target_dir: &Path,
    result: &mut DiffResult,
) -> Result<()> {
    let cursor_dir = target_dir.join(".cursor");

    // Rules + Memory → .cursorrules (merged)
    if !preset.rules.is_empty() || !preset.memory.is_empty() {
        let target_file = target_dir.join(".cursorrules");
        compare_file_exists(&target_file, ".cursorrules", result);
    }

    // Commands
    for file in &preset.commands {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let target_file = cursor_dir.join("commands").join(&*filename);
        compare_file(&target_file, &file.content, &format!("commands/{}", filename), result);
    }

    // MCP
    if !preset.mcp.is_empty() {
        let target_file = cursor_dir.join("mcp.json");
        compare_file_exists(&target_file, "mcp.json", result);
    }

    // Hooks
    if !preset.hooks.is_empty() {
        let target_file = cursor_dir.join("hooks.json");
        compare_file_exists(&target_file, "hooks.json", result);
    }

    // Agents
    for file in &preset.agents {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let target_file = cursor_dir.join("agents").join(&*filename);
        compare_file(&target_file, &file.content, &format!("agents/{}", filename), result);
    }

    // Skills
    for file in &preset.skills {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let target_file = cursor_dir.join("skills").join(&*filename);
        compare_file(&target_file, &file.content, &format!("skills/{}", filename), result);
    }

    Ok(())
}

/// Compute diff for GitHub Copilot
fn compute_copilot_diff(
    preset: &PresetFiles,
    target_dir: &Path,
    result: &mut DiffResult,
) -> Result<()> {
    let github_dir = target_dir.join(".github");

    // Rules + Memory → .github/copilot-instructions.md (merged)
    if !preset.rules.is_empty() || !preset.memory.is_empty() {
        let target_file = github_dir.join("copilot-instructions.md");
        compare_file_exists(&target_file, "copilot-instructions.md", result);
    }

    // Commands → .github/prompts/*.prompt.md
    for file in &preset.commands {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let prompt_name = filename.replace(".md", ".prompt.md");
        let target_file = github_dir.join("prompts").join(&prompt_name);
        compare_file(&target_file, &file.content, &format!("prompts/{}", prompt_name), result);
    }

    // MCP → .vscode/mcp.json
    if !preset.mcp.is_empty() {
        let target_file = target_dir.join(".vscode").join("mcp.json");
        compare_file_exists(&target_file, ".vscode/mcp.json", result);
    }

    // Agents → .github/agents/*.agent.md
    for file in &preset.agents {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let agent_name = filename.replace(".md", ".agent.md");
        let target_file = github_dir.join("agents").join(&agent_name);
        compare_file(&target_file, &file.content, &format!("agents/{}", agent_name), result);
    }

    // Skills → .github/skills/
    for file in &preset.skills {
        let filename = Path::new(&file.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let target_file = github_dir.join("skills").join(&*filename);
        compare_file(&target_file, &file.content, &format!("skills/{}", filename), result);
    }

    Ok(())
}

/// Compare a single file
fn compare_file(target_path: &Path, preset_content: &str, display_name: &str, result: &mut DiffResult) {
    if !target_path.exists() {
        result.new_files.push(display_name.to_string());
        return;
    }

    if let Ok(existing_content) = fs::read_to_string(target_path) {
        let existing_normalized = normalize_content(&existing_content);
        let preset_normalized = normalize_content(preset_content);

        if existing_normalized == preset_normalized {
            result.unchanged_files.push(display_name.to_string());
        } else {
            // Calculate simple diff info
            let existing_lines = existing_content.lines().count();
            let preset_lines = preset_content.lines().count();
            let diff_info = if preset_lines > existing_lines {
                format!("+{} lines", preset_lines - existing_lines)
            } else if existing_lines > preset_lines {
                format!("-{} lines", existing_lines - preset_lines)
            } else {
                "content differs".to_string()
            };
            result.modified_files.push((display_name.to_string(), Some(diff_info)));
        }
    } else {
        result.new_files.push(display_name.to_string());
    }
}

/// Compare file existence only (for merged files)
fn compare_file_exists(target_path: &Path, display_name: &str, result: &mut DiffResult) {
    if target_path.exists() {
        result.modified_files.push((display_name.to_string(), Some("will be updated".to_string())));
    } else {
        result.new_files.push(display_name.to_string());
    }
}

/// Normalize content for comparison (trim whitespace, normalize line endings)
fn normalize_content(content: &str) -> String {
    content
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
