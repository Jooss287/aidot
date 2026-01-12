use crate::error::Result;
use crate::template::config::MergeStrategy;
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

    /// Preview what changes would be made (for dry-run mode)
    fn preview(
        &self,
        template_files: &TemplateFiles,
        target_dir: &Path,
    ) -> PreviewResult;
}

/// Template files organized by section with merge strategies
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

    // Merge strategies for each section
    pub rules_strategy: MergeStrategy,
    pub memory_strategy: MergeStrategy,
    pub commands_strategy: MergeStrategy,
    pub mcp_strategy: MergeStrategy,
    pub hooks_strategy: MergeStrategy,
    pub agents_strategy: MergeStrategy,
    pub skills_strategy: MergeStrategy,
    pub settings_strategy: MergeStrategy,
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

    #[allow(dead_code)]
    pub fn add_skipped(&mut self, path: String) {
        self.skipped.push(path);
    }

    /// Check if any changes would be made
    #[allow(dead_code)]
    pub fn has_changes(&self) -> bool {
        !self.created.is_empty() || !self.updated.is_empty()
    }
}

/// Preview result for dry-run mode
#[derive(Debug)]
pub struct PreviewResult {
    /// Files that would be created
    pub would_create: Vec<PreviewFile>,
    /// Files that would be updated
    pub would_update: Vec<PreviewFile>,
    /// Files that would be skipped
    pub would_skip: Vec<String>,
}

#[derive(Debug)]
pub struct PreviewFile {
    pub path: String,
    pub section: String,
}

impl PreviewResult {
    pub fn new() -> Self {
        Self {
            would_create: Vec::new(),
            would_update: Vec::new(),
            would_skip: Vec::new(),
        }
    }

    pub fn add_would_create(&mut self, path: String, section: String) {
        self.would_create.push(PreviewFile { path, section });
    }

    pub fn add_would_update(&mut self, path: String, section: String) {
        self.would_update.push(PreviewFile { path, section });
    }

    #[allow(dead_code)]
    pub fn add_would_skip(&mut self, path: String) {
        self.would_skip.push(path);
    }

    pub fn has_changes(&self) -> bool {
        !self.would_create.is_empty() || !self.would_update.is_empty()
    }
}
