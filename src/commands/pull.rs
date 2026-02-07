use crate::adapters::traits::{ApplyResult, PendingChange};
use crate::adapters::{
    all_tools, detect_tools, normalize_content, write_with_conflict, ConflictMode,
};
use crate::error::Result;
use crate::preset::parse_preset;
use crate::repository;
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;

/// Pull and apply preset configurations
pub fn pull_preset(
    preset_source: String,
    tools_filter: Option<Vec<String>>,
    dry_run: bool,
    force: bool,
    skip: bool,
) -> Result<()> {
    // Resolve repository source (local path, Git URL, or registered repo name)
    let preset_path = repository::resolve_repository_source(&preset_source)?;

    println!(
        "{} {}",
        "Loading preset from".cyan(),
        preset_path.display().to_string().white()
    );

    // Parse preset
    let (_config, preset_files) = parse_preset(&preset_path)?;

    // Get current directory as target
    let target_dir = std::env::current_dir()?;

    // Detect or create tools based on --tools filter
    let tools = if let Some(ref filter) = tools_filter {
        // When --tools is specified, use all adapters (bypass detection)
        // so users can deploy to tools that haven't been set up yet
        let all = all_tools(&target_dir);
        let filter_lower: Vec<String> = filter.iter().map(|s| s.to_lowercase()).collect();
        let filtered: Vec<_> = all
            .into_iter()
            .filter(|tool| {
                let tool_name = tool.name().to_lowercase();
                filter_lower.iter().any(|f| {
                    tool_name.contains(f)
                        || f.contains(&tool_name)
                        || match f.as_str() {
                            "claude" => tool_name.contains("claude"),
                            "cursor" => tool_name.contains("cursor"),
                            "copilot" => tool_name.contains("copilot"),
                            _ => false,
                        }
                })
            })
            .collect();

        if filtered.is_empty() {
            println!(
                "{} {}",
                "No tools matched the filter:".yellow(),
                filter.join(", ").white()
            );
            return Ok(());
        }
        filtered
    } else {
        let detected = detect_tools(&target_dir);
        if detected.is_empty() {
            println!("{}", "No LLM tools detected in current directory.".yellow());
            println!("Run '{}' to see detection details.", "aidot detect".cyan());
            return Ok(());
        }
        detected
    };

    println!(
        "{} {} {}",
        "Detected".green(),
        tools.len().to_string().white().bold(),
        "tool(s):".green()
    );
    for tool in &tools {
        println!("  {} {}", "-".cyan(), tool.name().white());
    }
    println!();

    // Phase 1: Scan all tools for changes
    println!("{}", "Scanning...".cyan());

    let mut all_changes: Vec<(String, PendingChange)> = Vec::new();

    // Scan root files first (tool-agnostic)
    for root_file in &preset_files.root {
        let target_path = target_dir.join(&root_file.relative_path);
        let (is_conflict, is_identical) = if target_path.exists() {
            let is_identical = match std::fs::read_to_string(&target_path) {
                Ok(existing) => {
                    normalize_content(&existing) == normalize_content(&root_file.content)
                }
                Err(_) => false,
            };
            (true, is_identical)
        } else {
            (false, false)
        };
        all_changes.push((
            "Root".to_string(),
            PendingChange {
                path: root_file.relative_path.clone(),
                section: "root".to_string(),
                is_conflict,
                is_identical,
            },
        ));
    }

    // Scan tool-specific files
    for tool in &tools {
        let scan_result = tool.scan(&preset_files, &target_dir);
        for change in scan_result.changes {
            all_changes.push((tool.name().to_string(), change));
        }
    }

    if all_changes.is_empty() {
        println!("{}", "No changes to apply.".yellow());
        return Ok(());
    }

    // Phase 2: Display changes
    println!();
    println!("{}", "Changes to apply:".white().bold());

    let creates: Vec<_> = all_changes.iter().filter(|(_, c)| !c.is_conflict).collect();
    let conflicts: Vec<_> = all_changes
        .iter()
        .filter(|(_, c)| c.is_conflict && !c.is_identical)
        .collect();
    let unchanged: Vec<_> = all_changes.iter().filter(|(_, c)| c.is_identical).collect();

    for (tool_name, change) in &creates {
        println!(
            "  {} {} {} {}",
            "CREATE".green().bold(),
            change.path.white(),
            format!("({})", change.section).dimmed(),
            format!("[{}]", tool_name).dimmed()
        );
    }

    for (tool_name, change) in &conflicts {
        println!(
            "  {} {} {} {} {}",
            "UPDATE".yellow().bold(),
            change.path.white(),
            "(conflict)".red(),
            format!("({})", change.section).dimmed(),
            format!("[{}]", tool_name).dimmed()
        );
    }

    for (tool_name, change) in &unchanged {
        println!(
            "  {} {} {} {}",
            "UNCHANGED".dimmed(),
            change.path.dimmed(),
            format!("({})", change.section).dimmed(),
            format!("[{}]", tool_name).dimmed()
        );
    }

    println!();

    // Phase 3: Handle dry-run mode
    if dry_run {
        println!("{}", "═══ DRY RUN MODE ═══".yellow().bold());
        if !conflicts.is_empty() {
            let mut summary_parts = vec![format!("{} conflict(s) found.", conflicts.len())];
            if !unchanged.is_empty() {
                summary_parts.push(format!("{} file(s) unchanged.", unchanged.len()));
            }
            println!(
                "{} {}",
                summary_parts.join(" ").yellow(),
                "Run without --dry-run to apply.".cyan()
            );
        } else if !unchanged.is_empty() {
            println!(
                "{} {}",
                format!("No conflicts. {} file(s) unchanged.", unchanged.len()).green(),
                "Run without --dry-run to apply.".cyan()
            );
        } else {
            println!("{}", "No conflicts. Run without --dry-run to apply.".cyan());
        }
        return Ok(());
    }

    // Phase 4: Determine conflict mode
    let conflict_mode = if force {
        ConflictMode::Force
    } else if skip {
        ConflictMode::Skip
    } else if conflicts.is_empty() {
        // No conflicts, proceed directly
        ConflictMode::Force
    } else {
        // Ask user how to handle conflicts
        ask_conflict_resolution(conflicts.len())?
    };

    // Phase 5: Apply changes
    println!("{}", "Applying...".cyan());

    // Apply root files first (tool-agnostic)
    if !preset_files.root.is_empty() {
        let root_result = apply_root_files(&preset_files.root, &target_dir, conflict_mode)?;
        print_apply_result("Root", &root_result);
    }

    // Apply tool-specific files
    for tool in tools {
        let result = tool.apply(&preset_files, &target_dir, conflict_mode)?;
        print_apply_result(tool.name(), &result);
    }

    println!();
    println!("{}", "Preset applied successfully!".green().bold());

    Ok(())
}

/// Print apply result for a tool or root
fn print_apply_result(name: &str, result: &ApplyResult) {
    let has_changes = !result.created.is_empty()
        || !result.updated.is_empty()
        || !result.skipped.is_empty()
        || !result.unchanged.is_empty();

    if has_changes {
        println!("\n{} {}", "Applied to".cyan(), name.white().bold());

        if !result.created.is_empty() {
            println!("  {}:", "Created".green());
            for file in &result.created {
                println!("    {} {}", "+".green(), file.white());
            }
        }

        if !result.updated.is_empty() {
            println!("  {}:", "Updated".yellow());
            for file in &result.updated {
                println!("    {} {}", "~".yellow(), file.white());
            }
        }

        if !result.skipped.is_empty() {
            println!("  {}:", "Skipped".dimmed());
            for file in &result.skipped {
                println!("    {} {}", "-".dimmed(), file.dimmed());
            }
        }

        if !result.unchanged.is_empty() {
            println!("  {}:", "Unchanged".dimmed());
            for file in &result.unchanged {
                println!("    {} {}", "=".dimmed(), file.dimmed());
            }
        }
    }
}

/// Apply root files directly to target directory
fn apply_root_files(
    root_files: &[crate::adapters::traits::PresetFile],
    target_dir: &Path,
    conflict_mode: ConflictMode,
) -> Result<ApplyResult> {
    let mut result = ApplyResult::new();
    let mut current_mode = conflict_mode;

    for file in root_files {
        let target_path = target_dir.join(&file.relative_path);

        current_mode = write_with_conflict(
            &target_path,
            &file.content,
            current_mode,
            &mut result,
            &file.relative_path,
        )?;
    }

    Ok(result)
}

/// Ask user how to handle conflicts
fn ask_conflict_resolution(conflict_count: usize) -> Result<ConflictMode> {
    println!(
        "{} {} {}",
        conflict_count.to_string().yellow().bold(),
        "conflict(s) found.".yellow(),
        "How to proceed?".white()
    );
    println!("  [f]orce    - overwrite all conflicts");
    println!("  [s]kip     - skip all conflicts, create new files only");
    println!("  [i]nteract - decide for each file");
    println!("  [c]ancel   - abort operation");
    print!("\n{} ", "Your choice:".cyan());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim().to_lowercase().as_str() {
        "f" | "force" => Ok(ConflictMode::Force),
        "s" | "skip" => Ok(ConflictMode::Skip),
        "i" | "interact" | "interactive" => Ok(ConflictMode::Ask),
        "c" | "cancel" | "" => {
            println!("{}", "Operation cancelled.".yellow());
            std::process::exit(0);
        }
        _ => {
            println!("{}", "Invalid choice. Cancelling.".red());
            std::process::exit(1);
        }
    }
}
