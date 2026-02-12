/// Check if content starts with YAML front matter (---\n...\n---)
pub fn has_frontmatter(content: &str) -> bool {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return false;
    }
    // Find the closing --- after the opening one
    let after_opening = &trimmed[3..];
    // Must have a newline after opening ---
    if !after_opening.starts_with('\n') && !after_opening.starts_with("\r\n") {
        return false;
    }
    // Find closing ---
    after_opening
        .find("\n---")
        .map(|pos| {
            let after_close = &after_opening[pos + 4..];
            after_close.is_empty()
                || after_close.starts_with('\n')
                || after_close.starts_with("\r\n")
        })
        .unwrap_or(false)
}

/// Strip section prefix from a preset file's relative path
///
/// # Examples
/// ```
/// strip_section_prefix("rules/code-style.md", "rules") // → "code-style.md"
/// strip_section_prefix("commands/build.md", "commands") // → "build.md"
/// ```
pub fn strip_section_prefix(relative_path: &str, section: &str) -> String {
    relative_path
        .replace(&format!("{}/", section), "")
        .replace(&format!("{}\\", section), "")
}

/// Insert a suffix before the `.md` extension in a filename
///
/// Returns `{filename}.{suffix}.md` even when the `.md` extension is absent.
///
/// # Examples
/// ```
/// add_suffix_before_ext("build.md", "prompt")           // → "build.prompt.md"
/// add_suffix_before_ext("code-style.md", "instructions") // → "code-style.instructions.md"
/// add_suffix_before_ext("readme", "prompt")              // → "readme.prompt.md"
/// ```
pub fn add_suffix_before_ext(filename: &str, suffix: &str) -> String {
    if let Some(stem) = filename.strip_suffix(".md") {
        format!("{}.{}.md", stem, suffix)
    } else {
        format!("{}.{}.md", filename, suffix)
    }
}

/// Convert a specific key to another key within YAML front matter
///
/// Returns the original content unchanged if no front matter exists.
/// Handles both `from_key:` and `from_key :` forms.
///
/// # Examples
/// ```
/// // "globs: **/*.rs" → "applyTo: **/*.rs"
/// convert_frontmatter_key(content, "globs", "applyTo")
/// ```
pub fn convert_frontmatter_key(content: &str, from_key: &str, to_key: &str) -> String {
    if !has_frontmatter(content) {
        return content.to_string();
    }

    let trimmed = content.trim_start();
    let after_opening = &trimmed[3..];
    if let Some(close_pos) = after_opening.find("\n---") {
        let frontmatter = &after_opening[..close_pos + 1];
        let rest = &after_opening[close_pos + 1..];

        let from_colon = format!("{}:", from_key);
        let from_space_colon = format!("{} :", from_key);

        let converted_frontmatter = frontmatter
            .lines()
            .map(|line| {
                if line.starts_with(&from_colon) || line.starts_with(&from_space_colon) {
                    line.replacen(from_key, to_key, 1)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("---{}{}", converted_frontmatter, rest)
    } else {
        content.to_string()
    }
}

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

/// Check if a command is available on the system
pub fn is_command_available(cmd_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    let check = std::process::Command::new("where").arg(cmd_name).output();
    #[cfg(not(target_os = "windows"))]
    let check = std::process::Command::new("which").arg(cmd_name).output();
    check.map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_frontmatter_valid() {
        assert!(has_frontmatter("---\ntitle: test\n---\n# Content"));
        assert!(has_frontmatter(
            "---\ndescription: rule\nglobs: \"**/*.rs\"\n---\n# Rule"
        ));
    }

    #[test]
    fn test_has_frontmatter_invalid() {
        // No front matter
        assert!(!has_frontmatter("# Just a heading"));
        // Starts with --- but no closing ---
        assert!(!has_frontmatter("---\ntitle: test\n# Content"));
        // Empty string
        assert!(!has_frontmatter(""));
        // No newline immediately after ---
        assert!(!has_frontmatter("---title: test\n---\n# Content"));
    }

    #[test]
    fn test_has_frontmatter_windows_line_endings() {
        assert!(has_frontmatter("---\r\ntitle: test\r\n---\r\n# Content"));
    }

    #[test]
    fn test_strip_section_prefix_unix() {
        assert_eq!(
            strip_section_prefix("rules/code-style.md", "rules"),
            "code-style.md"
        );
        assert_eq!(
            strip_section_prefix("commands/build.md", "commands"),
            "build.md"
        );
        assert_eq!(
            strip_section_prefix("agents/helper.md", "agents"),
            "helper.md"
        );
    }

    #[test]
    fn test_strip_section_prefix_windows() {
        assert_eq!(
            strip_section_prefix("rules\\code-style.md", "rules"),
            "code-style.md"
        );
        assert_eq!(
            strip_section_prefix("commands\\build.md", "commands"),
            "build.md"
        );
    }

    #[test]
    fn test_add_suffix_before_ext_with_md() {
        assert_eq!(
            add_suffix_before_ext("build.md", "prompt"),
            "build.prompt.md"
        );
        assert_eq!(
            add_suffix_before_ext("code-style.md", "instructions"),
            "code-style.instructions.md"
        );
        assert_eq!(add_suffix_before_ext("agent.md", "agent"), "agent.agent.md");
    }

    #[test]
    fn test_add_suffix_before_ext_without_md() {
        assert_eq!(
            add_suffix_before_ext("readme", "prompt"),
            "readme.prompt.md"
        );
        assert_eq!(
            add_suffix_before_ext("config.txt", "instructions"),
            "config.txt.instructions.md"
        );
    }

    #[test]
    fn test_convert_frontmatter_key_basic() {
        let input = "---\ndescription: Rust rules\nglobs: \"**/*.rs\"\n---\n# Content";
        let result = convert_frontmatter_key(input, "globs", "applyTo");
        assert!(result.contains("applyTo: \"**/*.rs\""));
        assert!(!result.contains("globs:"));
        assert!(result.contains("# Content"));
    }

    #[test]
    fn test_convert_frontmatter_key_no_frontmatter() {
        let input = "# Just content\nNo frontmatter here.";
        let result = convert_frontmatter_key(input, "globs", "applyTo");
        assert_eq!(result, input);
    }

    #[test]
    fn test_convert_frontmatter_key_missing_key() {
        let input = "---\ndescription: test\n---\n# Content";
        let result = convert_frontmatter_key(input, "globs", "applyTo");
        // No globs key, so content remains unchanged
        assert!(result.contains("description: test"));
        assert!(!result.contains("applyTo"));
    }

    #[test]
    fn test_convert_frontmatter_key_with_space() {
        let input = "---\nglobs : \"**/*.rs\"\n---\n# Content";
        let result = convert_frontmatter_key(input, "globs", "applyTo");
        assert!(result.contains("applyTo :"));
        assert!(!result.contains("globs"));
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
}
