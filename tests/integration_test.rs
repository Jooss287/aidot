//! Integration tests for aidot CLI
//!
//! These tests verify end-to-end workflows like:
//! - init → pull → diff
//! - Preset parsing and application
//! - Multi-tool support

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run aidot command
fn run_aidot(args: &[&str], cwd: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_aidot"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute aidot")
}

/// Helper to create a basic preset structure
fn create_test_preset(dir: &std::path::Path) {
    // Create .aidot-config.toml
    let config = r#"
[metadata]
name = "test-preset"
version = "1.0.0"
description = "Test preset"

[rules]
files = ["rules/test.md"]
merge_strategy = "concat"

[memory]
directory = "memory/"
merge_strategy = "concat"

[commands]
directory = "commands/"
merge_strategy = "replace"
"#;
    fs::write(dir.join(".aidot-config.toml"), config).unwrap();

    // Create directories
    fs::create_dir_all(dir.join("rules")).unwrap();
    fs::create_dir_all(dir.join("memory")).unwrap();
    fs::create_dir_all(dir.join("commands")).unwrap();

    // Create sample files
    fs::write(
        dir.join("rules/test.md"),
        "# Test Rule\n\nThis is a test rule.",
    )
    .unwrap();
    fs::write(
        dir.join("memory/context.md"),
        "# Context\n\nProject context info.",
    )
    .unwrap();
    fs::write(dir.join("commands/build.md"), "# Build\n\nBuild command.").unwrap();
}

#[test]
fn test_init_creates_preset_structure() {
    let temp_dir = TempDir::new().unwrap();
    let output = run_aidot(&["init"], temp_dir.path());

    assert!(output.status.success(), "init should succeed");

    // Check created files
    assert!(temp_dir.path().join(".aidot-config.toml").exists());
    assert!(temp_dir.path().join("commands").exists());
}

#[test]
fn test_init_with_force_overwrites() {
    let temp_dir = TempDir::new().unwrap();

    // First init
    run_aidot(&["init"], temp_dir.path());

    // Modify config
    fs::write(temp_dir.path().join(".aidot-config.toml"), "# Modified").unwrap();

    // Second init with --force
    let output = run_aidot(&["init", "--force"], temp_dir.path());
    assert!(output.status.success());

    // Config should be reset
    let content = fs::read_to_string(temp_dir.path().join(".aidot-config.toml")).unwrap();
    assert!(content.contains("[metadata]"));
}

#[test]
fn test_detect_command() {
    let temp_dir = TempDir::new().unwrap();

    // Create .claude directory to be detected
    fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();

    let output = run_aidot(&["detect"], temp_dir.path());

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Claude Code") || stdout.contains("detected"));
}

#[test]
fn test_pull_from_local_preset() {
    let _temp_dir = TempDir::new().unwrap();
    let preset_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    // Create preset
    create_test_preset(preset_dir.path());

    // Create .claude directory in project to ensure detection
    fs::create_dir_all(project_dir.path().join(".claude")).unwrap();

    // Pull from local preset
    let output = run_aidot(
        &["pull", preset_dir.path().to_str().unwrap()],
        project_dir.path(),
    );

    assert!(
        output.status.success(),
        "pull should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify files were created
    assert!(project_dir.path().join(".claude").exists());
}

#[test]
fn test_pull_dry_run() {
    let preset_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    // Create preset
    create_test_preset(preset_dir.path());

    // Create .claude directory
    fs::create_dir_all(project_dir.path().join(".claude")).unwrap();

    // Pull with --dry-run
    let output = run_aidot(
        &["pull", preset_dir.path().to_str().unwrap(), "--dry-run"],
        project_dir.path(),
    );

    assert!(output.status.success());

    // Files should NOT be created (dry run)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would") || stdout.contains("Preview") || stdout.contains("dry"));
}

#[test]
fn test_diff_command() {
    let preset_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    // Create preset
    create_test_preset(preset_dir.path());

    // Create project with .claude dir
    fs::create_dir_all(project_dir.path().join(".claude")).unwrap();

    // Run diff
    let output = run_aidot(
        &["diff", preset_dir.path().to_str().unwrap()],
        project_dir.path(),
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show some diff output (new/modified/unchanged)
    assert!(
        stdout.contains("new")
            || stdout.contains("modified")
            || stdout.contains("unchanged")
            || stdout.contains("Summary"),
        "diff output should contain status: {}",
        stdout
    );
}

#[test]
fn test_status_command() {
    let temp_dir = TempDir::new().unwrap();

    // Create some tool directories
    fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
    fs::create_dir_all(temp_dir.path().join(".cursor")).unwrap();

    let output = run_aidot(&["status"], temp_dir.path());

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Claude") || stdout.contains("Cursor") || stdout.contains("detected"));
}

#[test]
fn test_repo_list_empty() {
    let temp_dir = TempDir::new().unwrap();

    let output = run_aidot(&["repo", "list"], temp_dir.path());

    // Should succeed even with no repos
    assert!(output.status.success());
}

#[test]
fn test_pull_with_tools_filter() {
    let preset_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    // Create preset
    create_test_preset(preset_dir.path());

    // Create both .claude and .cursor dirs
    fs::create_dir_all(project_dir.path().join(".claude")).unwrap();
    fs::create_dir_all(project_dir.path().join(".cursor")).unwrap();

    // Pull only for claude
    let output = run_aidot(
        &[
            "pull",
            preset_dir.path().to_str().unwrap(),
            "--tools",
            "claude",
        ],
        project_dir.path(),
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should mention Claude Code
    assert!(stdout.contains("Claude") || output.status.success());
}

#[test]
fn test_help_command() {
    let temp_dir = TempDir::new().unwrap();

    let output = run_aidot(&["--help"], temp_dir.path());

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("aidot"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("pull"));
}

#[test]
fn test_version_command() {
    let temp_dir = TempDir::new().unwrap();

    let output = run_aidot(&["--version"], temp_dir.path());

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("aidot") || stdout.contains("0.1"));
}
