use crate::commands::pull_preset;
use crate::error::{AidotError, Result};
use crate::repository::is_git_url;
use colored::Colorize;
use std::path::Path;

/// Pull and apply a preset once without registering the repository.
///
/// The source must be a Git URL (https://, git@, …) or a local path.
/// For registered repository names use `aidot pull <name>` instead.
pub fn quick_pull(
    source: String,
    tools_filter: Option<Vec<String>>,
    dry_run: bool,
    force: bool,
    skip: bool,
) -> Result<()> {
    // Validate: only Git URLs and existing local paths are accepted.
    // Registered repository names are intentionally excluded – users should
    // use `aidot pull <name>` for those.
    let is_local = Path::new(&source).exists();
    if !is_git_url(&source) && !is_local {
        return Err(AidotError::RepositoryNotFound(format!(
            "'{}' is not a Git URL or a local path.\n\
             \n\
             {}  Use a Git URL:   aidot quick-pull https://github.com/user/preset\n\
             {}  Use a local path: aidot quick-pull ./my-preset\n\
             {}  For registered repos use: aidot pull {}",
            source,
            "→".cyan(),
            "→".cyan(),
            "→".cyan(),
            source,
        )));
    }

    println!(
        "{} {}",
        "Quick-pull (one-off, no registration):".cyan().bold(),
        source.white()
    );

    pull_preset(source, tools_filter, dry_run, force, skip)
}
