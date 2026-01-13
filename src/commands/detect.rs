use crate::adapters::detector::get_detected_tool_names;
use crate::error::Result;
use colored::Colorize;
use std::env;

/// Detect installed LLM tools
pub fn detect_tools() -> Result<()> {
    let current_dir = env::current_dir()?;

    println!(
        "{} {}\n",
        "Detecting LLM tools in".cyan(),
        current_dir.display().to_string().white()
    );

    let tools = get_detected_tool_names(&current_dir);

    let mut detected_count = 0;

    for tool in &tools {
        if tool.detected {
            println!(
                "  {} {} {}",
                "✓".green(),
                tool.name.white().bold(),
                "(detected)".green()
            );
            detected_count += 1;
        } else {
            println!(
                "  {} {} {}",
                "✗".red(),
                tool.name.dimmed(),
                "(not detected)".dimmed()
            );
        }
    }

    println!();

    if detected_count == 0 {
        println!("{}", "No LLM tools detected.".yellow());
        println!(
            "\n{}",
            "To use aidot, you need at least one supported LLM tool:".dimmed()
        );
        println!(
            "  {} Create a {} directory or install {} CLI",
            "•".cyan(),
            ".claude".white(),
            "claude".white()
        );
        println!("  {} Use {} IDE", "•".cyan(), "Cursor".white());
        println!(
            "  {} Use VS Code with {}",
            "•".cyan(),
            "GitHub Copilot".white()
        );
    } else {
        println!(
            "{} {} {}",
            "Found".green(),
            detected_count.to_string().white().bold(),
            "tool(s).".green()
        );
    }

    Ok(())
}
