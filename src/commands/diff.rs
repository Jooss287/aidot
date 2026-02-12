use crate::adapters::detector::detect_tools;
use crate::adapters::normalize_content;
use crate::error::Result;
use crate::preset::parser::parse_preset;
use crate::repository::resolve_repository_source;
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

        // Use scan() to compute changes (handles all adapter-specific logic)
        let scan = tool.scan(&preset_files, &target_dir);

        if scan.changes.is_empty() {
            println!("  {} No preset files for this tool\n", "○".dimmed());
            continue;
        }

        let mut new_files = Vec::new();
        let mut modified_files: Vec<(String, Option<String>)> = Vec::new();
        let mut unchanged_files = Vec::new();

        for change in &scan.changes {
            if !change.is_conflict {
                // File doesn't exist → new
                new_files.push(change.path.clone());
            } else if change.is_identical {
                // File exists with same content → unchanged
                unchanged_files.push(change.path.clone());
            } else {
                // File exists with different content → modified
                let diff_info = change
                    .preset_content
                    .as_ref()
                    .and_then(|preset_content| {
                        let full_path = target_dir.join(&change.path);
                        compute_diff_info(&full_path, preset_content)
                    })
                    .or_else(|| Some("will be updated".to_string()));
                modified_files.push((change.path.clone(), diff_info));
            }
        }

        // New files (would be created)
        if !new_files.is_empty() {
            println!("  {} New files:", "+".green().bold());
            for file in &new_files {
                println!("    {} {}", "+".green(), file.white());
            }
            total_new += new_files.len();
        }

        // Modified files (content differs)
        if !modified_files.is_empty() {
            println!("  {} Modified files:", "~".yellow().bold());
            for (file, diff_info) in &modified_files {
                println!("    {} {}", "~".yellow(), file.white());
                if let Some(info) = diff_info {
                    println!("      {}", info.dimmed());
                }
            }
            total_modified += modified_files.len();
        }

        // Unchanged files
        if !unchanged_files.is_empty() {
            println!("  {} Unchanged files:", "=".dimmed());
            for file in &unchanged_files {
                println!("    {} {}", "=".dimmed(), file.dimmed());
            }
            total_unchanged += unchanged_files.len();
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

/// Compute diff info (line count comparison) between an existing file and preset content
fn compute_diff_info(target_path: &Path, preset_content: &str) -> Option<String> {
    let existing_content = fs::read_to_string(target_path).ok()?;
    let existing_normalized = normalize_content(&existing_content);
    let preset_normalized = normalize_content(preset_content);

    if existing_normalized == preset_normalized {
        return None;
    }

    let existing_lines = existing_content.lines().count();
    let preset_lines = preset_content.lines().count();
    Some(if preset_lines > existing_lines {
        format!("+{} lines", preset_lines - existing_lines)
    } else if existing_lines > preset_lines {
        format!("-{} lines", existing_lines - preset_lines)
    } else {
        "content differs".to_string()
    })
}
