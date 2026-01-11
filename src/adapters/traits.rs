use crate::error::Result;
use std::path::Path;

/// Represents a template file to be converted
#[derive(Debug, Clone)]
pub struct TemplateFile {
    /// Relative path from template root (e.g., "rules/code-style.md")
    pub relative_path: String,
    /// Full content of the file
    pub content: String,
}

/// Trait for LLM tool adapters
pub trait ToolAdapter {
    /// Get the name of the tool (e.g., "Claude Code", "Cursor")
    fn name(&self) -> &str;

    /// Detect if this tool is available/installed
    fn detect(&self) -> bool;

    /// Apply template files to the target project directory
    ///
    /// # Arguments
    /// * `template_files` - Map of section name to list of files
    ///   - "rules" -> Vec<TemplateFile>
    ///   - "memory" -> Vec<TemplateFile>
    ///   - "commands" -> Vec<TemplateFile>
    ///   - etc.
    /// * `target_dir` - Project directory where files should be written
    /// * `force` - Overwrite existing files without asking
    fn apply(
        &self,
        template_files: &TemplateFiles,
        target_dir: &Path,
        force: bool,
    ) -> Result<ApplyResult>;
}

/// Template files organized by section
#[derive(Debug, Default)]
pub struct TemplateFiles {
    pub rules: Vec<TemplateFile>,
    pub memory: Vec<TemplateFile>,
    pub commands: Vec<TemplateFile>,
    pub mcp: Vec<TemplateFile>,
    pub hooks: Vec<TemplateFile>,
    pub agents: Vec<TemplateFile>,
    pub skills: Vec<TemplateFile>,
    pub settings: Vec<TemplateFile>,
}

/// Result of applying a template
#[derive(Debug)]
pub struct ApplyResult {
    /// Files that were created
    pub created: Vec<String>,
    /// Files that were updated
    pub updated: Vec<String>,
    /// Files that were skipped (already exist and force=false)
    pub skipped: Vec<String>,
}

impl ApplyResult {
    pub fn new() -> Self {
        Self {
            created: Vec::new(),
            updated: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn add_created(&mut self, path: String) {
        self.created.push(path);
    }

    pub fn add_updated(&mut self, path: String) {
        self.updated.push(path);
    }

    pub fn add_skipped(&mut self, path: String) {
        self.skipped.push(path);
    }
}
