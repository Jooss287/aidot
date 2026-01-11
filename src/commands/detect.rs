use crate::adapters::detector::get_detected_tool_names;
use crate::error::Result;
use std::env;

/// Detect installed LLM tools
pub fn detect_tools() -> Result<()> {
    let current_dir = env::current_dir()?;

    println!("Detecting LLM tools in {}...\n", current_dir.display());

    let tools = get_detected_tool_names(&current_dir);

    let mut detected_count = 0;

    for tool in &tools {
        if tool.detected {
            println!("  ✓ {} (detected)", tool.name);
            detected_count += 1;
        } else {
            println!("  ✗ {} (not detected)", tool.name);
        }
    }

    println!();

    if detected_count == 0 {
        println!("No LLM tools detected.");
        println!("\nTo use aidot, you need at least one supported LLM tool:");
        println!("  - Claude Code: Create a .claude directory or install claude CLI");
        println!("  - Cursor: Use Cursor IDE");
        println!("  - Aider: Install aider CLI");
        println!("  - GitHub Copilot: Use VS Code with Copilot");
    } else {
        println!("Found {} tool(s).", detected_count);
    }

    Ok(())
}
