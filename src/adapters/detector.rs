use super::claude_code::ClaudeCodeAdapter;
use super::copilot::CopilotAdapter;
use super::cursor::CursorAdapter;
use super::ToolAdapter;
use std::path::Path;

/// Detected tool information
#[derive(Debug, Clone)]
pub struct DetectedTool {
    pub name: String,
    pub detected: bool,
}

/// Detect all available LLM tools in the current directory
pub fn detect_tools(project_dir: &Path) -> Vec<Box<dyn ToolAdapter>> {
    let mut tools: Vec<Box<dyn ToolAdapter>> = Vec::new();

    // Check Claude Code
    let claude_adapter = ClaudeCodeAdapter::new(project_dir);
    if claude_adapter.detect() {
        tools.push(Box::new(claude_adapter));
    }

    // Check Cursor
    let cursor_adapter = CursorAdapter::new(project_dir);
    if cursor_adapter.detect() {
        tools.push(Box::new(cursor_adapter));
    }

    // Check GitHub Copilot
    let copilot_adapter = CopilotAdapter::new(project_dir);
    if copilot_adapter.detect() {
        tools.push(Box::new(copilot_adapter));
    }

    tools
}

/// Get list of detected tool names
pub fn get_detected_tool_names(project_dir: &Path) -> Vec<DetectedTool> {
    let claude_adapter = ClaudeCodeAdapter::new(project_dir);
    let cursor_adapter = CursorAdapter::new(project_dir);
    let copilot_adapter = CopilotAdapter::new(project_dir);

    vec![
        DetectedTool {
            name: "Claude Code".to_string(),
            detected: claude_adapter.detect(),
        },
        DetectedTool {
            name: "Cursor".to_string(),
            detected: cursor_adapter.detect(),
        },
        DetectedTool {
            name: "GitHub Copilot".to_string(),
            detected: copilot_adapter.detect(),
        },
    ]
}
