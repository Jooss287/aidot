use crate::error::{AidotError, Result};
use git2::{FetchOptions, RemoteCallbacks, Repository};
use std::path::Path;

/// Clone a Git repository to the specified path
pub fn clone_repository(url: &str, target_path: &Path) -> Result<()> {
    println!("Cloning repository from {}...", url);

    // Setup callbacks for progress
    let mut callbacks = RemoteCallbacks::new();
    callbacks.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            print!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            print!(
                "Received {}/{} objects ({} bytes)\r",
                stats.received_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
        }
        std::io::Write::flush(&mut std::io::stdout()).ok();
        true
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);

    match builder.clone(url, target_path) {
        Ok(_) => {
            println!("\n✓ Repository cloned successfully");
            Ok(())
        }
        Err(e) => Err(AidotError::Git(format!(
            "Failed to clone repository: {}",
            e
        ))),
    }
}

/// Pull latest changes from a Git repository
pub fn pull_repository(repo_path: &Path) -> Result<()> {
    println!("Updating repository at {}...", repo_path.display());

    let repo = Repository::open(repo_path)
        .map_err(|e| AidotError::Git(format!("Failed to open repository: {}", e)))?;

    // Fetch from origin
    let mut remote = repo
        .find_remote("origin")
        .map_err(|e| AidotError::Git(format!("Failed to find remote 'origin': {}", e)))?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            print!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            print!(
                "Received {}/{} objects\r",
                stats.received_objects(),
                stats.total_objects()
            );
        }
        std::io::Write::flush(&mut std::io::stdout()).ok();
        true
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    remote
        .fetch(&["main", "master"], Some(&mut fetch_options), None)
        .map_err(|e| AidotError::Git(format!("Failed to fetch: {}", e)))?;

    // Get the fetch head and merge
    let fetch_head = repo
        .find_reference("FETCH_HEAD")
        .map_err(|e| AidotError::Git(format!("Failed to find FETCH_HEAD: {}", e)))?;

    let fetch_commit = repo
        .reference_to_annotated_commit(&fetch_head)
        .map_err(|e| AidotError::Git(format!("Failed to get commit: {}", e)))?;

    // Perform fast-forward merge
    let analysis = repo
        .merge_analysis(&[&fetch_commit])
        .map_err(|e| AidotError::Git(format!("Failed to analyze merge: {}", e)))?;

    if analysis.0.is_up_to_date() {
        println!("Already up-to-date");
    } else if analysis.0.is_fast_forward() {
        // Fast-forward merge
        let refname = "refs/heads/main"; // or master
        let mut reference = repo
            .find_reference(refname)
            .or_else(|_| repo.find_reference("refs/heads/master"))
            .map_err(|e| AidotError::Git(format!("Failed to find branch: {}", e)))?;

        reference
            .set_target(fetch_commit.id(), "Fast-forward")
            .map_err(|e| AidotError::Git(format!("Failed to set target: {}", e)))?;

        repo.set_head(reference.name().unwrap())
            .map_err(|e| AidotError::Git(format!("Failed to set HEAD: {}", e)))?;

        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| AidotError::Git(format!("Failed to checkout: {}", e)))?;

        println!("\n✓ Repository updated successfully");
    } else {
        return Err(AidotError::Git(
            "Cannot fast-forward, manual merge required".to_string(),
        ));
    }

    Ok(())
}

/// Check if a path is a valid Git repository
pub fn is_git_repository(path: &Path) -> bool {
    Repository::open(path).is_ok()
}
