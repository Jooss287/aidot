use super::conflict::ConflictMode;
use super::helpers::normalize_content;
use crate::error::Result;
use std::path::Path;

/// Represents a preset file to be converted
#[derive(Debug, Clone)]
pub struct PresetFile {
    /// Relative path from preset root (e.g., "rules/code-style.md")
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

    /// Scan for changes without applying them
    /// Returns a list of pending changes with conflict information
    fn scan(&self, preset_files: &PresetFiles, target_dir: &Path) -> ScanResult;

    /// Apply preset files to the target project directory
    ///
    /// # Arguments
    /// * `preset_files` - Map of section name to list of files
    ///   - "rules" -> Vec<PresetFile>
    ///   - "memory" -> Vec<PresetFile>
    ///   - "commands" -> Vec<PresetFile>
    ///   - etc.
    /// * `target_dir` - Project directory where files should be written
    /// * `conflict_mode` - How to handle existing files
    fn apply(
        &self,
        preset_files: &PresetFiles,
        target_dir: &Path,
        conflict_mode: &mut ConflictMode,
    ) -> Result<ApplyResult>;
}

/// Preset files organized by section
#[derive(Debug, Default)]
pub struct PresetFiles {
    pub rules: Vec<PresetFile>,
    pub memory: Vec<PresetFile>,
    pub commands: Vec<PresetFile>,
    pub mcp: Vec<PresetFile>,
    pub hooks: Vec<PresetFile>,
    pub agents: Vec<PresetFile>,
    pub skills: Vec<PresetFile>,
    pub settings: Vec<PresetFile>,
    pub root: Vec<PresetFile>,
}

/// A pending change detected during scan
#[derive(Debug, Clone)]
pub struct PendingChange {
    /// Display path (e.g., ".claude/CLAUDE.md")
    pub path: String,
    /// Section name (e.g., "rules", "memory")
    pub section: String,
    /// Whether this is a conflict (file already exists)
    pub is_conflict: bool,
    /// Whether the file exists AND content is identical (normalized comparison)
    pub is_identical: bool,
    /// Preset content for diff display (None for merged files like memory/mcp)
    pub preset_content: Option<String>,
}

/// Result of scanning for changes
#[derive(Debug, Default)]
pub struct ScanResult {
    pub changes: Vec<PendingChange>,
}

impl ScanResult {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    pub fn add_change(&mut self, path: String, section: String, is_conflict: bool) {
        self.changes.push(PendingChange {
            path,
            section,
            is_conflict,
            is_identical: false,
            preset_content: None,
        });
    }

    /// Add a change with content-based comparison for 1:1 file mappings.
    pub fn add_change_with_content(
        &mut self,
        path: String,
        section: String,
        target_path: &Path,
        preset_content: &str,
    ) {
        if target_path.exists() {
            let is_identical = match std::fs::read_to_string(target_path) {
                Ok(existing) => normalize_content(&existing) == normalize_content(preset_content),
                Err(_) => false,
            };
            self.changes.push(PendingChange {
                path,
                section,
                is_conflict: true,
                is_identical,
                preset_content: Some(preset_content.to_string()),
            });
        } else {
            self.changes.push(PendingChange {
                path,
                section,
                is_conflict: false,
                is_identical: false,
                preset_content: Some(preset_content.to_string()),
            });
        }
    }
}

#[cfg(test)]
impl ScanResult {
    pub fn conflicts(&self) -> Vec<&PendingChange> {
        self.changes
            .iter()
            .filter(|c| c.is_conflict && !c.is_identical)
            .collect()
    }

    pub fn creates(&self) -> Vec<&PendingChange> {
        self.changes.iter().filter(|c| !c.is_conflict).collect()
    }

    pub fn identical(&self) -> Vec<&PendingChange> {
        self.changes.iter().filter(|c| c.is_identical).collect()
    }

    pub fn has_conflicts(&self) -> bool {
        self.changes
            .iter()
            .any(|c| c.is_conflict && !c.is_identical)
    }

    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }
}

/// Result of applying a preset
#[derive(Debug)]
pub struct ApplyResult {
    /// Files that were created
    pub created: Vec<String>,
    /// Files that were updated
    pub updated: Vec<String>,
    /// Files that were skipped (already exist and force=false)
    pub skipped: Vec<String>,
    /// Files that were identical (auto-skipped)
    pub unchanged: Vec<String>,
}

impl ApplyResult {
    pub fn new() -> Self {
        Self {
            created: Vec::new(),
            updated: Vec::new(),
            skipped: Vec::new(),
            unchanged: Vec::new(),
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

    pub fn add_unchanged(&mut self, path: String) {
        self.unchanged.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_file_creation() {
        let file = PresetFile {
            relative_path: "rules/code-style.md".to_string(),
            content: "# Code Style Rules".to_string(),
        };
        assert_eq!(file.relative_path, "rules/code-style.md");
        assert_eq!(file.content, "# Code Style Rules");
    }

    #[test]
    fn test_preset_files_default() {
        let files = PresetFiles::default();
        assert!(files.rules.is_empty());
        assert!(files.memory.is_empty());
        assert!(files.commands.is_empty());
        assert!(files.mcp.is_empty());
        assert!(files.hooks.is_empty());
        assert!(files.agents.is_empty());
        assert!(files.skills.is_empty());
        assert!(files.settings.is_empty());
        assert!(files.root.is_empty());
    }

    #[test]
    fn test_apply_result() {
        let mut result = ApplyResult::new();
        assert!(result.created.is_empty());
        assert!(result.updated.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.unchanged.is_empty());

        result.add_created("file1.md".to_string());
        assert_eq!(result.created.len(), 1);

        result.add_updated("file2.md".to_string());
        assert_eq!(result.updated.len(), 1);

        result.add_skipped("file3.md".to_string());
        assert_eq!(result.skipped.len(), 1);

        result.add_unchanged("file4.md".to_string());
        assert_eq!(result.unchanged.len(), 1);
    }

    #[test]
    fn test_scan_result() {
        let mut result = ScanResult::new();
        assert!(!result.has_changes());
        assert!(!result.has_conflicts());

        result.add_change("file1.md".to_string(), "rules".to_string(), false);
        assert!(result.has_changes());
        assert!(!result.has_conflicts());
        assert_eq!(result.creates().len(), 1);
        assert_eq!(result.conflicts().len(), 0);
        assert_eq!(result.identical().len(), 0);

        result.add_change("file2.md".to_string(), "memory".to_string(), true);
        assert!(result.has_conflicts());
        assert_eq!(result.creates().len(), 1);
        assert_eq!(result.conflicts().len(), 1);
    }

    #[test]
    fn test_scan_result_with_identical() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "# Test Content\n").unwrap();

        let mut result = ScanResult::new();

        // Same content -> is_identical should be true
        result.add_change_with_content(
            "test.md".to_string(),
            "rules".to_string(),
            &file_path,
            "# Test Content\n",
        );
        assert_eq!(result.identical().len(), 1);
        assert_eq!(result.conflicts().len(), 0); // identical files are not conflicts

        // Different content -> is_identical should be false
        result.add_change_with_content(
            "test2.md".to_string(),
            "rules".to_string(),
            &file_path,
            "# Different Content\n",
        );
        assert_eq!(result.identical().len(), 1);
        assert_eq!(result.conflicts().len(), 1);

        // Non-existent file -> create, not conflict
        let missing_path = temp_dir.path().join("missing.md");
        result.add_change_with_content(
            "missing.md".to_string(),
            "rules".to_string(),
            &missing_path,
            "# New Content\n",
        );
        assert_eq!(result.creates().len(), 1);
    }
}
