use crate::error::{AidotError, Result};
use git2::Repository;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// Cache for git availability check
static GIT_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if git CLI is available on the system
pub fn check_git_available() -> Result<()> {
    let is_available = GIT_AVAILABLE.get_or_init(|| {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    });

    if *is_available {
        Ok(())
    } else {
        Err(AidotError::Git(
            "Git is not installed or not found in PATH. Please install git to use remote repositories.\n\
             - Windows: https://git-scm.com/download/win\n\
             - macOS: brew install git\n\
             - Linux: apt install git / dnf install git".to_string()
        ))
    }
}

/// Clone a Git repository using system git CLI
/// This provides better compatibility with SSH agents, credential helpers, and various auth methods
fn clone_with_git_cli(url: &str, target_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["clone", "--progress", url])
        .arg(target_path)
        .output()
        .map_err(|e| {
            AidotError::Git(format!(
                "Failed to execute git command. Is git installed? Error: {}",
                e
            ))
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(AidotError::Git(format!(
            "Failed to clone repository: {}",
            stderr
        )))
    }
}

/// Pull latest changes using system git CLI
fn pull_with_git_cli(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| {
            AidotError::Git(format!(
                "Failed to execute git command. Is git installed? Error: {}",
                e
            ))
        })?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Already up to date") || stdout.contains("Already up-to-date") {
            println!("Already up-to-date");
        } else {
            println!("{}", stdout);
        }
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(AidotError::Git(format!("Failed to pull: {}", stderr)))
    }
}

/// Clone a Git repository to the specified path
pub fn clone_repository(url: &str, target_path: &Path) -> Result<()> {
    check_git_available()?;
    println!("Cloning repository from {}...", url);

    // Use system git CLI for better SSH/auth compatibility
    clone_with_git_cli(url, target_path)?;
    println!("Repository cloned successfully");
    Ok(())
}

/// Pull latest changes from a Git repository
pub fn pull_repository(repo_path: &Path) -> Result<()> {
    check_git_available()?;
    println!("Updating repository at {}...", repo_path.display());

    // Use system git CLI for better SSH/auth compatibility
    pull_with_git_cli(repo_path)?;
    println!("Repository updated successfully");
    Ok(())
}

/// Check if a path is a valid Git repository
pub fn is_git_repository(path: &Path) -> bool {
    Repository::open(path).is_ok()
}
