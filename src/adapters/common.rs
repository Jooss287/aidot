use super::conflict::{write_with_conflict, ConflictMode};
use super::helpers::strip_section_prefix;
use super::traits::{ApplyResult, PresetFile, ScanResult};
use crate::error::Result;
use std::fs;
use std::path::Path;

/// Transform function that receives (stripped_filename, content) and returns the final filename
pub type FilenameTransform<'a> = Option<&'a dyn Fn(&str, &str) -> String>;

/// Transform function that receives content and returns transformed content
pub type ContentTransform<'a> = Option<&'a dyn Fn(&str) -> String>;

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir(dir: &Path) -> Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

/// Apply 1:1 mapped files (commands, agents, skills, or rules without special transforms)
///
/// Each preset file in the section is written to `target_dir` with optional filename/content transforms.
///
/// - `section`: section name in preset (e.g., "commands", "agents", "skills")
/// - `target_dir`: directory to write files into
/// - `display_prefix`: prefix for display paths (e.g., ".claude/commands")
/// - `filename_transform`: optional function to transform the stripped filename (receives stripped name and content)
/// - `content_transform`: optional function to transform file content before writing
#[allow(clippy::too_many_arguments)]
pub fn apply_one_to_one(
    files: &[PresetFile],
    section: &str,
    target_dir: &Path,
    display_prefix: &str,
    result: &mut ApplyResult,
    mode: &mut ConflictMode,
    filename_transform: FilenameTransform<'_>,
    content_transform: ContentTransform<'_>,
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(target_dir)?;

    for file in files {
        let stripped = strip_section_prefix(&file.relative_path, section);
        let filename = match filename_transform {
            Some(transform) => transform(&stripped, &file.content),
            None => stripped,
        };
        let target_path = target_dir.join(&filename);
        let display_path = format!("{}/{}", display_prefix, filename);

        let content = match content_transform {
            Some(transform) => transform(&file.content),
            None => file.content.clone(),
        };

        write_with_conflict(&target_path, &content, mode, result, &display_path)?;
    }

    Ok(())
}

/// Scan 1:1 mapped files for changes
///
/// - `section`: section name in preset (e.g., "rules", "commands")
/// - `target_dir`: directory where files would be written
/// - `display_prefix`: prefix for display paths
/// - `filename_transform`: optional function to transform the stripped filename
/// - `content_transform`: optional function to transform file content for comparison
pub fn scan_one_to_one(
    files: &[PresetFile],
    section: &str,
    target_dir: &Path,
    display_prefix: &str,
    scan_result: &mut ScanResult,
    filename_transform: FilenameTransform<'_>,
    content_transform: ContentTransform<'_>,
) {
    for file in files {
        let stripped = strip_section_prefix(&file.relative_path, section);
        let filename = match filename_transform {
            Some(transform) => transform(&stripped, &file.content),
            None => stripped,
        };
        let target_path = target_dir.join(&filename);
        let display_path = format!("{}/{}", display_prefix, filename);

        let content = match content_transform {
            Some(transform) => transform(&file.content),
            None => file.content.clone(),
        };

        scan_result.add_change_with_content(
            display_path,
            section.to_string(),
            &target_path,
            &content,
        );
    }
}

/// Apply JSON files by merging them into a single object under a wrapper key
///
/// Each preset file's JSON content is inserted as `wrapper_key[server_name]`.
/// - `target_path`: path to the merged JSON file
/// - `display_path`: display path for conflict messages
/// - `section`: section name to strip prefix from filenames
/// - `wrapper_key`: JSON key to nest servers under (e.g., "mcpServers", "servers")
/// - `default_json`: initial JSON value if file doesn't exist
#[allow(clippy::too_many_arguments)]
pub fn apply_json_merge(
    files: &[PresetFile],
    section: &str,
    target_path: &Path,
    display_path: &str,
    wrapper_key: &str,
    default_json: serde_json::Value,
    result: &mut ApplyResult,
    mode: &mut ConflictMode,
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    // Ensure parent directory exists
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read existing or use default
    let mut config: serde_json::Value = if target_path.exists() {
        let content = fs::read_to_string(target_path)?;
        serde_json::from_str(&content)?
    } else {
        default_json
    };

    // Ensure wrapper key exists
    if config.get(wrapper_key).is_none() {
        config[wrapper_key] = serde_json::json!({});
    }

    // Merge each file's content
    for file in files {
        let entry_name = strip_section_prefix(&file.relative_path, section).replace(".json", "");
        let entry_config: serde_json::Value = serde_json::from_str(&file.content)?;
        config[wrapper_key][entry_name] = entry_config;
    }

    let json_str = serde_json::to_string_pretty(&config)?;
    write_with_conflict(target_path, &json_str, mode, result, display_path)?;

    Ok(())
}

/// Scan a merged JSON section for changes (mcp, hooks, settings)
pub fn scan_merged_section(
    files: &[PresetFile],
    display_path: &str,
    section: &str,
    target_path: &Path,
    scan_result: &mut ScanResult,
) {
    if !files.is_empty() {
        scan_result.add_change(
            display_path.to_string(),
            section.to_string(),
            target_path.exists(),
        );
    }
}
