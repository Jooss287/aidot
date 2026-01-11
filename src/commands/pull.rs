use crate::adapters::{detect_tools, ToolAdapter};
use crate::error::Result;
use crate::template::parse_template;
use std::path::{Path, PathBuf};

/// Pull and apply template configurations
pub fn pull_template(
    template_source: String,
    _tools_filter: Option<Vec<String>>,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    // For now, treat source as a local directory path
    let template_path = PathBuf::from(&template_source);

    if !template_path.exists() {
        return Err(crate::error::AidotError::InvalidTemplate(format!(
            "Template path does not exist: {}",
            template_path.display()
        )));
    }

    println!("Loading template from {}...", template_path.display());

    // Parse template
    let (_config, template_files) = parse_template(&template_path)?;

    // Get current directory as target
    let target_dir = std::env::current_dir()?;

    // Detect available tools
    let tools = detect_tools(&target_dir);

    if tools.is_empty() {
        println!("No LLM tools detected in current directory.");
        println!("Run 'aidot detect' to see detection details.");
        return Ok(());
    }

    println!("Detected {} tool(s):", tools.len());
    for tool in &tools {
        println!("  - {}", tool.name());
    }
    println!();

    if dry_run {
        println!("[DRY RUN] No files will be modified.");
        return Ok(());
    }

    // Apply to each detected tool
    for tool in tools {
        println!("Applying to {}...", tool.name());

        let result = tool.apply(&template_files, &target_dir, force)?;

        // Print results
        if !result.created.is_empty() {
            println!("  Created:");
            for file in &result.created {
                println!("    ✓ {}", file);
            }
        }

        if !result.updated.is_empty() {
            println!("  Updated:");
            for file in &result.updated {
                println!("    ✓ {}", file);
            }
        }

        if !result.skipped.is_empty() {
            println!("  Skipped:");
            for file in &result.skipped {
                println!("    - {}", file);
            }
        }

        println!();
    }

    println!("✓ Template applied successfully!");

    Ok(())
}
