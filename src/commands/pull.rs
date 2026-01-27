use crate::adapters::traits::PendingChange;
use crate::adapters::{detect_tools, ConflictMode};
use crate::error::Result;
use crate::preset::parse_preset;
use crate::repository;
use colored::Colorize;
use std::io::{self, Write};

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

    // Detect available tools
    let mut tools = detect_tools(&target_dir);

    if tools.is_empty() {
        println!("{}", "No LLM tools detected in current directory.".yellow());
        println!("Run '{}' to see detection details.", "aidot detect".cyan());
        return Ok(());
    }

    // Filter tools if specified
    if let Some(ref filter) = tools_filter {
        let filter_lower: Vec<String> = filter.iter().map(|s| s.to_lowercase()).collect();
        tools.retain(|tool| {
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
        });

        if tools.is_empty() {
            println!(
                "{} {}",
                "No tools matched the filter:".yellow(),
                filter.join(", ").white()
            );
            return Ok(());
        }
    }

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
    let conflicts: Vec<_> = all_changes.iter().filter(|(_, c)| c.is_conflict).collect();

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

    println!();

    // Phase 3: Handle dry-run mode
    if dry_run {
        println!("{}", "═══ DRY RUN MODE ═══".yellow().bold());
        if !conflicts.is_empty() {
            println!(
                "{} {} {}",
                conflicts.len().to_string().yellow().bold(),
                "conflict(s) found.".yellow(),
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

    for tool in tools {
        let result = tool.apply(&preset_files, &target_dir, conflict_mode)?;

        let has_changes =
            !result.created.is_empty() || !result.updated.is_empty() || !result.skipped.is_empty();

        if has_changes {
            println!("\n{} {}", "Applied to".cyan(), tool.name().white().bold());

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
        }
    }

    println!();
    println!("{}", "Preset applied successfully!".green().bold());

    Ok(())
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
