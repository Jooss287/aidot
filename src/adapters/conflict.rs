use super::helpers::normalize_content;
use super::traits::ApplyResult;
use std::collections::HashMap;
use std::path::Path;

/// How to handle file conflicts during apply
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ConflictMode {
    /// Overwrite existing files without asking
    Force,
    /// Skip existing files without asking
    Skip,
    /// Ask user for each conflict (default)
    #[default]
    Ask,
    /// Pre-resolved decisions (display_path → should_write)
    /// Results from pre-resolving all conflicts when interactive mode is chosen
    /// fallback_all: default behavior for files not in the decision map (None=inline prompt, Some(true)=overwrite, Some(false)=skip)
    PreResolved {
        decisions: HashMap<String, bool>,
        fallback_all: Option<bool>,
    },
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
    /// Returns whether the file should be written.
    /// May mutate self (e.g., Ask → Force when user chooses "Overwrite All").
    /// `existing_content` and `new_content` enable diff display in interactive mode.
    pub fn resolve_conflict(
        &mut self,
        file_path: &str,
        existing_content: Option<&str>,
        new_content: Option<&str>,
    ) -> bool {
        match self {
            ConflictMode::Force => true,
            ConflictMode::Skip => false,
            ConflictMode::PreResolved {
                decisions,
                fallback_all,
            } => {
                match decisions.get(file_path).copied() {
                    Some(should_write) => should_write,
                    None => match *fallback_all {
                        // Previously selected OverwriteAll/SkipAll
                        Some(should_write) => should_write,
                        // Files that can't be pre-resolved (e.g., merged files): handle inline
                        None => {
                            let diff_available =
                                existing_content.is_some() && new_content.is_some();
                            if let (Some(existing), Some(new)) = (existing_content, new_content) {
                                Self::print_diff(file_path, existing, new);
                            }
                            loop {
                                let decision = Self::ask_user(file_path, diff_available);
                                match decision {
                                    ConflictDecision::Overwrite => return true,
                                    ConflictDecision::Skip => return false,
                                    ConflictDecision::OverwriteAll => {
                                        *fallback_all = Some(true);
                                        return true;
                                    }
                                    ConflictDecision::SkipAll => {
                                        *fallback_all = Some(false);
                                        return false;
                                    }
                                    ConflictDecision::ShowDiff => {
                                        if let (Some(existing), Some(new)) =
                                            (existing_content, new_content)
                                        {
                                            Self::print_diff(file_path, existing, new);
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }
            ConflictMode::Ask => {
                let diff_available = existing_content.is_some() && new_content.is_some();
                // Auto-show diff first if available
                if let (Some(existing), Some(new)) = (existing_content, new_content) {
                    Self::print_diff(file_path, existing, new);
                }
                loop {
                    let decision = Self::ask_user(file_path, diff_available);
                    match decision {
                        ConflictDecision::Overwrite => return true,
                        ConflictDecision::Skip => return false,
                        ConflictDecision::OverwriteAll => {
                            *self = ConflictMode::Force;
                            return true;
                        }
                        ConflictDecision::SkipAll => {
                            *self = ConflictMode::Skip;
                            return false;
                        }
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
    pub fn ask_user(file_path: &str, diff_available: bool) -> ConflictDecision {
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
    pub fn print_diff(file_path: &str, existing: &str, new: &str) {
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
/// Mutates `mode` in place (e.g., Ask → Force when user chooses "Overwrite All")
pub fn write_with_conflict(
    target_path: &Path,
    content: &str,
    mode: &mut ConflictMode,
    result: &mut ApplyResult,
    display_path: &str,
) -> std::io::Result<()> {
    use std::fs;

    if target_path.exists() {
        // Read existing content for both comparison and diff display
        let existing_content = fs::read_to_string(target_path).ok();

        // Content comparison: auto-skip if identical
        if let Some(ref existing) = existing_content {
            if normalize_content(existing) == normalize_content(content) {
                result.add_unchanged(display_path.to_string());
                return Ok(());
            }
        }

        let should_write =
            mode.resolve_conflict(display_path, existing_content.as_deref(), Some(content));
        if should_write {
            fs::write(target_path, content)?;
            result.add_updated(display_path.to_string());
        } else {
            result.add_skipped(display_path.to_string());
        }
        Ok(())
    } else {
        // Create parent directories if needed
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, content)?;
        result.add_created(display_path.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_mode_force() {
        let mut mode = ConflictMode::Force;
        let should_write = mode.resolve_conflict("test.md", None, None);
        assert!(should_write);
        assert_eq!(mode, ConflictMode::Force);
    }

    #[test]
    fn test_conflict_mode_skip() {
        let mut mode = ConflictMode::Skip;
        let should_write = mode.resolve_conflict("test.md", None, None);
        assert!(!should_write);
        assert_eq!(mode, ConflictMode::Skip);
    }

    #[test]
    fn test_conflict_mode_pre_resolved() {
        let mut decisions = HashMap::new();
        decisions.insert("file1.md".to_string(), true);
        decisions.insert("file2.md".to_string(), false);

        let mut mode = ConflictMode::PreResolved {
            decisions,
            fallback_all: None,
        };

        // Look up pre-resolved decisions
        assert!(mode.resolve_conflict("file1.md", None, None));
        assert!(!mode.resolve_conflict("file2.md", None, None));
        // Files not in the decision map fall back to inline Ask
        // (Cannot test in unit tests as it requires stdin)
    }

    #[test]
    fn test_write_with_conflict_identical_auto_skip() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let content = "# Test Content\n";
        std::fs::write(&file_path, content).unwrap();

        let mut result = ApplyResult::new();
        let mut mode = ConflictMode::Force;
        write_with_conflict(&file_path, content, &mut mode, &mut result, "test.md").unwrap();

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
        let mut mode = ConflictMode::Force;
        write_with_conflict(
            &file_path,
            "# New Content\n",
            &mut mode,
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
        let mut mode = ConflictMode::Force;
        write_with_conflict(&file_path, "# New File\n", &mut mode, &mut result, "new.md").unwrap();

        assert_eq!(result.created.len(), 1);
        assert_eq!(mode, ConflictMode::Force);
    }
}
