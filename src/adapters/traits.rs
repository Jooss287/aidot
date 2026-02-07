use crate::error::Result;
use std::path::Path;

/// Normalize content for comparison (trim trailing whitespace, normalize line endings)
pub fn normalize_content(content: &str) -> String {
    content
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

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
    /// Show diff between local and remote
    ShowDiff,
}

impl ConflictMode {
    /// Resolve how to handle a conflict for a specific file.
    /// Returns (should_write, updated_mode) where updated_mode may change to Force/Skip
    /// if user chose "all" option.
    /// `existing_content` and `new_content` enable diff display in interactive mode.
    pub fn resolve_conflict(
        &self,
        file_path: &str,
        existing_content: Option<&str>,
        new_content: Option<&str>,
    ) -> (bool, ConflictMode) {
        match self {
            ConflictMode::Force => (true, ConflictMode::Force),
            ConflictMode::Skip => (false, ConflictMode::Skip),
            ConflictMode::Ask => {
                let diff_available = existing_content.is_some() && new_content.is_some();
                loop {
                    let decision = Self::ask_user(file_path, diff_available);
                    match decision {
                        ConflictDecision::Overwrite => return (true, ConflictMode::Ask),
                        ConflictDecision::Skip => return (false, ConflictMode::Ask),
                        ConflictDecision::OverwriteAll => return (true, ConflictMode::Force),
                        ConflictDecision::SkipAll => return (false, ConflictMode::Skip),
                        ConflictDecision::ShowDiff => {
                            if let (Some(existing), Some(new)) = (existing_content, new_content) {
                                Self::print_diff(file_path, existing, new);
                            }
                            // Loop back to ask again
                        }
                    }
                }
            }
        }
    }

    /// Ask user what to do with a conflicting file
    fn ask_user(file_path: &str, diff_available: bool) -> ConflictDecision {
        use colored::Colorize;
        use std::io::{self, Write};

        loop {
            if diff_available {
                print!(
                    "  {} '{}' {} [o]verwrite / [s]kip / [d]iff / [O]verwrite all / [S]kip all? ",
                    "Conflict:".yellow(),
                    file_path,
                    "already exists.".dimmed()
                );
            } else {
                print!(
                    "  {} '{}' {} [o]verwrite / [s]kip / [O]verwrite all / [S]kip all? ",
                    "Conflict:".yellow(),
                    file_path,
                    "already exists.".dimmed()
                );
            }
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                // If we can't read input, default to skip
                return ConflictDecision::Skip;
            }

            match input.trim() {
                "o" | "y" | "yes" => return ConflictDecision::Overwrite,
                "s" | "n" | "no" => return ConflictDecision::Skip,
                "d" if diff_available => return ConflictDecision::ShowDiff,
                "O" | "a" | "all" => return ConflictDecision::OverwriteAll,
                "S" | "N" => return ConflictDecision::SkipAll,
                "" => return ConflictDecision::Skip, // Default to skip on Enter
                _ => {
                    if diff_available {
                        println!("  {} Please enter 'o', 's', 'd', 'O', or 'S'", "?".yellow());
                    } else {
                        println!("  {} Please enter 'o', 's', 'O', or 'S'", "?".yellow());
                    }
                }
            }
        }
    }

    /// Print unified diff between local and preset content
    fn print_diff(file_path: &str, existing: &str, new: &str) {
        use colored::Colorize;
        use similar::{ChangeTag, TextDiff};

        println!();
        println!("  {} {}", "--- (local)".red(), file_path.dimmed());
        println!("  {} {}", "+++ (preset)".green(), file_path.dimmed());

        let diff = TextDiff::from_lines(existing, new);

        for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
            println!("  {}", format!("{}", hunk.header()).cyan());
            for change in hunk.iter_changes() {
                let line = change.to_string_lossy();
                let line_trimmed = line.trim_end_matches('\n');
                match change.tag() {
                    ChangeTag::Delete => {
                        println!("  {}", format!("-{}", line_trimmed).red());
                    }
                    ChangeTag::Insert => {
                        println!("  {}", format!("+{}", line_trimmed).green());
                    }
                    ChangeTag::Equal => {
                        println!("  {}", format!(" {}", line_trimmed).dimmed());
                    }
                }
            }
        }
        println!();
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
        // Read existing content for both comparison and diff display
        let existing_content = fs::read_to_string(target_path).ok();

        // Content comparison: auto-skip if identical
        if let Some(ref existing) = existing_content {
            if normalize_content(existing) == normalize_content(content) {
                result.add_unchanged(display_path.to_string());
                return Ok(mode);
            }
        }

        let (should_write, new_mode) =
            mode.resolve_conflict(display_path, existing_content.as_deref(), Some(content));
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
    /// Whether the file exists AND content is identical (normalized comparison)
    pub is_identical: bool,
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
            });
        } else {
            self.changes.push(PendingChange {
                path,
                section,
                is_conflict: false,
                is_identical: false,
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

    #[test]
    fn test_write_with_conflict_identical_auto_skip() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let content = "# Test Content\n";
        std::fs::write(&file_path, content).unwrap();

        let mut result = ApplyResult::new();
        let mode = write_with_conflict(
            &file_path,
            content,
            ConflictMode::Force,
            &mut result,
            "test.md",
        )
        .unwrap();

        // Should be unchanged, not updated
        assert_eq!(result.unchanged.len(), 1);
        assert_eq!(result.updated.len(), 0);
        assert_eq!(mode, ConflictMode::Force);
    }

    #[test]
    fn test_write_with_conflict_different_content() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "# Old Content\n").unwrap();

        let mut result = ApplyResult::new();
        let mode = write_with_conflict(
            &file_path,
            "# New Content\n",
            ConflictMode::Force,
            &mut result,
            "test.md",
        )
        .unwrap();

        // Should be updated, not unchanged
        assert_eq!(result.updated.len(), 1);
        assert_eq!(result.unchanged.len(), 0);
        assert_eq!(mode, ConflictMode::Force);

        // Verify file was actually written
        let written = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, "# New Content\n");
    }

    #[test]
    fn test_write_with_conflict_new_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new.md");

        let mut result = ApplyResult::new();
        let mode = write_with_conflict(
            &file_path,
            "# New File\n",
            ConflictMode::Force,
            &mut result,
            "new.md",
        )
        .unwrap();

        assert_eq!(result.created.len(), 1);
        assert_eq!(mode, ConflictMode::Force);
    }

    #[test]
    fn test_normalize_content() {
        // Trailing whitespace normalization
        assert_eq!(
            normalize_content("hello  \nworld  "),
            normalize_content("hello\nworld")
        );
        // Trailing newline normalization
        assert_eq!(
            normalize_content("hello\nworld\n"),
            normalize_content("hello\nworld")
        );
        // Windows vs Unix line endings (lines() handles both)
        assert_eq!(
            normalize_content("hello\r\nworld"),
            normalize_content("hello\nworld")
        );
    }

    #[test]
    fn test_conflict_mode_force() {
        let mode = ConflictMode::Force;
        let (should_write, new_mode) = mode.resolve_conflict("test.md", None, None);
        assert!(should_write);
        assert_eq!(new_mode, ConflictMode::Force);
    }

    #[test]
    fn test_conflict_mode_skip() {
        let mode = ConflictMode::Skip;
        let (should_write, new_mode) = mode.resolve_conflict("test.md", None, None);
        assert!(!should_write);
        assert_eq!(new_mode, ConflictMode::Skip);
    }
}
