use crate::error::Result;
use colored::Colorize;

/// Build-time version from AIDOT_VERSION env var, falls back to Cargo.toml version
const VERSION: &str = match option_env!("AIDOT_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

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
    let current_version = VERSION.strip_prefix('v').unwrap_or(VERSION);
    let target = get_target();

    println!(
        "{} v{} ({})",
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

    // Build identifier pattern: aidot-v{version}-{target}.{ext}
    // self_update replaces {version} with version WITHOUT 'v' prefix (e.g., "0.1.0")
    // But our release assets have 'v' prefix (e.g., "aidot-v0.1.0-...")
    // So we add 'v' before {version} in the pattern
    let identifier = format!("aidot-v{{{{version}}}}-{}.{}", target, get_archive_ext());

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

    // self_update strips 'v' prefix from version field, but name keeps it
    // We need the tag with 'v' for API calls
    let latest_tag = &latest_release.name;
    // Version for comparison (without 'v')
    let latest_version = &latest_release.version;

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

    // Build expected asset name: aidot-v0.1.3-beta-x86_64-pc-windows-msvc.zip
    let expected_asset_name = format!("aidot-{}-{}.{}", latest_tag, target, get_archive_ext());

    // Find the matching asset in the release
    let asset = latest_release
        .assets
        .iter()
        .find(|a| a.name == expected_asset_name)
        .ok_or_else(|| {
            crate::error::AidotError::UpdateError(format!(
                "No asset found matching '{}'. Available assets: {:?}",
                expected_asset_name,
                latest_release
                    .assets
                    .iter()
                    .map(|a| &a.name)
                    .collect::<Vec<_>>()
            ))
        })?;

    // Download and extract using self_update's helper functions
    let tmp_dir = tempfile::Builder::new()
        .prefix("aidot-update")
        .tempdir()
        .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

    let tmp_archive_path = tmp_dir.path().join(&asset.name);

    // Build the browser download URL (direct download link)
    // Format: https://github.com/{owner}/{repo}/releases/download/{tag}/{asset_name}
    let download_url = format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        REPO_OWNER, REPO_NAME, latest_tag, asset.name
    );

    // Download with progress
    println!("{} {}", "Downloading:".dimmed(), download_url);
    let response = reqwest::blocking::get(&download_url)
        .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

    let bytes = response
        .bytes()
        .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

    std::fs::write(&tmp_archive_path, &bytes)
        .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

    // Extract the archive
    println!("{}", "Extracting...".dimmed());
    let tmp_extract_dir = tmp_dir.path().join("extracted");
    std::fs::create_dir_all(&tmp_extract_dir)
        .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

    #[cfg(target_os = "windows")]
    {
        let archive_file = std::fs::File::open(&tmp_archive_path)
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;
        let mut archive = zip::ZipArchive::new(archive_file)
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;
        archive
            .extract(&tmp_extract_dir)
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let archive_file = std::fs::File::open(&tmp_archive_path)
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;
        let decoder = flate2::read::GzDecoder::new(archive_file);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(&tmp_extract_dir)
            .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;
    }

    // Replace current binary
    let new_binary = tmp_extract_dir
        .join(BIN_NAME)
        .with_extension(std::env::consts::EXE_EXTENSION);

    println!("{}", "Installing...".dimmed());
    self_update::self_replace::self_replace(&new_binary)
        .map_err(|e| crate::error::AidotError::UpdateError(e.to_string()))?;

    println!();
    println!(
        "{} Updated to version {}",
        "✓".green().bold(),
        latest_version.green().bold()
    );

    Ok(())
}
