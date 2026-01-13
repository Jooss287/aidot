use crate::error::Result;
use colored::Colorize;

const REPO_OWNER: &str = "Jooss287";
const REPO_NAME: &str = "aidot";
const BIN_NAME: &str = "aidot";

/// Get the target triple for the current platform
fn get_target() -> &'static str {
    // Compile-time target detection
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "x86_64-unknown-linux-gnu"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "aarch64-unknown-linux-gnu"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "x86_64-apple-darwin"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "aarch64-apple-darwin"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "x86_64-pc-windows-msvc"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        compile_error!("Unsupported platform for self-update")
    }
}

/// Get the archive extension for the current platform
fn get_archive_ext() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "zip"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "tar.gz"
    }
}

/// Check for updates and optionally update to the latest version
pub fn check_update(check_only: bool, include_prerelease: bool) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let target = get_target();

    println!(
        "{} {} ({})",
        "Current version:".cyan(),
        current_version.white().bold(),
        target.dimmed()
    );

    if include_prerelease {
        println!(
            "{} {}",
            "Checking for updates".dimmed(),
            "(including prereleases)...".yellow()
        );
    } else {
        println!("{}", "Checking for updates...".dimmed());
    }

    // Build identifier pattern: aidot-{version}-{target}.{ext}
    // e.g., aidot-v0.1.0-x86_64-pc-windows-msvc.zip
    let identifier = format!("aidot-{{{{version}}}}-{}.{}", target, get_archive_ext());

    // Find latest release (stable or prerelease based on flag)
    let latest_release = if include_prerelease {
        // Fetch all releases and find the latest one (including prereleases)
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .build()
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?
            .fetch()
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

        releases
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::AidotError::UpdateError("No releases found".to_string()))?
    } else {
        // Use the default /releases/latest API (excludes prereleases)
        let update_builder = self_update::backends::github::Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name(BIN_NAME)
            .target(target)
            .identifier(&identifier)
            .current_version(current_version)
            .build()
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

        update_builder
            .get_latest_release()
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?
    };

    let latest_version = latest_release.version.trim_start_matches('v');

    if latest_version == current_version {
        println!(
            "{} {}",
            "✓".green(),
            "You are already on the latest version!".white()
        );
        return Ok(());
    }

    // Check if it's a prerelease
    let is_prerelease = latest_version.contains('-');
    let version_display = if is_prerelease {
        format!(
            "{} {}",
            latest_version.green().bold(),
            "(prerelease)".yellow()
        )
    } else {
        latest_version.green().bold().to_string()
    };

    println!(
        "{} {} → {}",
        "New version available:".yellow(),
        current_version.dimmed(),
        version_display
    );

    if check_only {
        println!();
        println!(
            "{}",
            "Run 'aidot update' to update to the latest version.".dimmed()
        );
        return Ok(());
    }

    println!();
    println!("{}", "Downloading update...".cyan());

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .target(target)
        .identifier(&identifier)
        .show_download_progress(true)
        .current_version(current_version)
        .target_version_tag(&latest_release.version)
        .build()
        .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?
        .update()
        .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

    println!();
    println!(
        "{} Updated to version {}",
        "✓".green().bold(),
        status.version().green().bold()
    );

    Ok(())
}
