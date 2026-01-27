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

/// How to handle file conflicts during apply
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ConflictMode {
    /// Overwrite existing files without asking
    Force,
    /// Skip existing files without asking
    Skip,
    /// Ask user for each conflict (default)
    #[default]
    Ask,
}

/// User's decision for a single conflict
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictDecision {
    /// Overwrite this file
    Overwrite,
    /// Skip this file
    Skip,
    /// Overwrite all remaining files
    OverwriteAll,
    /// Skip all remaining files
    SkipAll,
}

impl ConflictMode {
    /// Resolve how to handle a conflict for a specific file.
    /// Returns (should_write, updated_mode) where updated_mode may change to Force/Skip
    /// if user chose "all" option.
    pub fn resolve_conflict(&self, file_path: &str) -> (bool, ConflictMode) {
        match self {
            ConflictMode::Force => (true, ConflictMode::Force),
            ConflictMode::Skip => (false, ConflictMode::Skip),
            ConflictMode::Ask => {
                let decision = Self::ask_user(file_path);
                match decision {
                    ConflictDecision::Overwrite => (true, ConflictMode::Ask),
                    ConflictDecision::Skip => (false, ConflictMode::Ask),
                    ConflictDecision::OverwriteAll => (true, ConflictMode::Force),
                    ConflictDecision::SkipAll => (false, ConflictMode::Skip),
                }
            }
        }
    }

    /// Ask user what to do with a conflicting file
    fn ask_user(file_path: &str) -> ConflictDecision {
        use colored::Colorize;
        use std::io::{self, Write};

        loop {
            print!(
                "  {} '{}' {} [o]verwrite / [s]kip / [O]verwrite all / [S]kip all? ",
                "Conflict:".yellow(),
                file_path,
                "already exists.".dimmed()
            );
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                // If we can't read input, default to skip
                return ConflictDecision::Skip;
            }

            match input.trim() {
                "o" | "y" | "yes" => return ConflictDecision::Overwrite,
                "s" | "n" | "no" => return ConflictDecision::Skip,
                "O" | "a" | "all" => return ConflictDecision::OverwriteAll,
                "S" | "N" => return ConflictDecision::SkipAll,
                "" => return ConflictDecision::Skip, // Default to skip on Enter
                _ => {
                    println!("  {} Please enter 'o', 's', 'O', or 'S'", "?".yellow());
                }
            }
        }
    }
}

/// Helper to write a file with conflict resolution
/// Returns updated ConflictMode (may change if user chose "all" option)
pub fn write_with_conflict(
    target_path: &Path,
    content: &str,
    mode: ConflictMode,
    result: &mut ApplyResult,
    display_path: &str,
) -> std::io::Result<ConflictMode> {
    use std::fs;

    if target_path.exists() {
        let (should_write, new_mode) = mode.resolve_conflict(display_path);
        if should_write {
            fs::write(target_path, content)?;
            result.add_updated(display_path.to_string());
        } else {
            result.add_skipped(display_path.to_string());
        }
        Ok(new_mode)
    } else {
        // Create parent directories if needed
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, content)?;
        result.add_created(display_path.to_string());
        Ok(mode)
    }
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
        conflict_mode: ConflictMode,
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
        });
    }
}

#[cfg(test)]
impl ScanResult {
    pub fn conflicts(&self) -> Vec<&PendingChange> {
        self.changes.iter().filter(|c| c.is_conflict).collect()
    }

    pub fn creates(&self) -> Vec<&PendingChange> {
        self.changes.iter().filter(|c| !c.is_conflict).collect()
    }

    pub fn has_conflicts(&self) -> bool {
        self.changes.iter().any(|c| c.is_conflict)
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

        result.add_created("file1.md".to_string());
        assert_eq!(result.created.len(), 1);

        result.add_updated("file2.md".to_string());
        assert_eq!(result.updated.len(), 1);

        result.add_skipped("file3.md".to_string());
        assert_eq!(result.skipped.len(), 1);
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

        result.add_change("file2.md".to_string(), "memory".to_string(), true);
        assert!(result.has_conflicts());
        assert_eq!(result.creates().len(), 1);
        assert_eq!(result.conflicts().len(), 1);
    }

    #[test]
    fn test_conflict_mode_force() {
        let mode = ConflictMode::Force;
        let (should_write, new_mode) = mode.resolve_conflict("test.md");
        assert!(should_write);
        assert_eq!(new_mode, ConflictMode::Force);
    }

    #[test]
    fn test_conflict_mode_skip() {
        let mode = ConflictMode::Skip;
        let (should_write, new_mode) = mode.resolve_conflict("test.md");
        assert!(!should_write);
        assert_eq!(new_mode, ConflictMode::Skip);
    }
}
