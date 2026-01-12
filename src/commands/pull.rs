use crate::adapters::detect_tools;
use crate::error::Result;
use crate::repository;
use crate::template::parse_template;
use colored::Colorize;

/// Pull and apply template configurations
pub fn pull_template(
    template_source: String,
    tools_filter: Option<Vec<String>>,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    // Resolve repository source (local path, Git URL, or registered repo name)
    let template_path = repository::resolve_repository_source(&template_source)?;

    println!(
        "{} {}",
        "Loading template from".cyan(),
        template_path.display().to_string().white()
    );

    // Parse template
    let (_config, template_files) = parse_template(&template_path)?;

    // Get current directory as target
    let target_dir = std::env::current_dir()?;

    // Detect available tools
    let mut tools = detect_tools(&target_dir);

    if tools.is_empty() {
        println!(
            "{}",
            "No LLM tools detected in current directory.".yellow()
        );
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
        println!("  {} {}", "•".cyan(), tool.name().white());
    }
    println!();

    if dry_run {
        println!(
            "{}",
            "═══ DRY RUN MODE ═══".yellow().bold()
        );
        println!(
            "{}\n",
            "No files will be modified. Showing what would happen:".yellow()
        );

        for tool in &tools {
            println!(
                "{} {}",
                "Preview for:".cyan(),
                tool.name().white().bold()
            );

            let preview = tool.preview(&template_files, &target_dir);

            if !preview.has_changes() {
                println!("  {} No changes would be made", "ℹ".blue());
            } else {
                if !preview.would_create.is_empty() {
                    println!("  {} Would create:", "CREATE".green().bold());
                    for file in &preview.would_create {
                        println!(
                            "    {} {} {}",
                            "+".green(),
                            file.path.white(),
                            format!("({})", file.section).dimmed()
                        );
                    }
                }

                if !preview.would_update.is_empty() {
                    println!("  {} Would update:", "UPDATE".yellow().bold());
                    for file in &preview.would_update {
                        println!(
                            "    {} {} {}",
                            "~".yellow(),
                            file.path.white(),
                            format!("({})", file.section).dimmed()
                        );
                    }
                }

                if !preview.would_skip.is_empty() {
                    println!("  {} Would skip:", "SKIP".dimmed());
                    for path in &preview.would_skip {
                        println!("    {} {}", "-".dimmed(), path.dimmed());
                    }
                }
            }
            println!();
        }

        println!(
            "{}",
            "Run without --dry-run to apply these changes.".cyan()
        );
        return Ok(());
    }

    // Apply to each detected tool
    for tool in tools {
        println!(
            "{} {}",
            "Applying to".cyan(),
            tool.name().white().bold()
        );

        let result = tool.apply(&template_files, &target_dir, force)?;

        // Print results with colors
        if !result.created.is_empty() {
            println!("  {}:", "Created".green());
            for file in &result.created {
                println!("    {} {}", "✓".green(), file.white());
            }
        }

        if !result.updated.is_empty() {
            println!("  {}:", "Updated".yellow());
            for file in &result.updated {
                println!("    {} {}", "✓".yellow(), file.white());
            }
        }

        if !result.skipped.is_empty() {
            println!("  {}:", "Skipped".dimmed());
            for file in &result.skipped {
                println!("    {} {}", "-".dimmed(), file.dimmed());
            }
        }

        println!();
    }

    println!("{}", "✓ Template applied successfully!".green().bold());

    Ok(())
}
