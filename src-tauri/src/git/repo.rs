//! Opening and validating a repository path.

use std::path::Path;

use super::types::RepoInfo;
use super::{run_git, GitError, Result};

/// Validate that `path` is inside a git work tree and return its identity.
/// Accepts any path inside the repo and resolves to the work-tree root.
pub fn open_repo(path: &Path) -> Result<RepoInfo> {
    if !path.is_dir() {
        return Err(GitError {
            message: format!("not a directory: {}", path.display()),
            command: String::new(),
            exit_code: None,
        });
    }

    let toplevel = run_git(path, &["rev-parse", "--show-toplevel"])?;
    let toplevel = toplevel.trim().to_string();

    let head = run_git(path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        // An unborn branch (fresh init, no commits) still names its branch
        // via the symbolic ref even though rev-parse HEAD fails.
        .or_else(|_| {
            run_git(path, &["symbolic-ref", "--short", "HEAD"]).map(|s| s.trim().to_string())
        })?;

    let is_detached = head == "HEAD";
    let head = if is_detached {
        run_git(path, &["rev-parse", "--short", "HEAD"])?
            .trim()
            .to_string()
    } else {
        head
    };

    let name = Path::new(&toplevel)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| toplevel.clone());

    Ok(RepoInfo {
        path: toplevel,
        name,
        head,
        is_detached,
    })
}

/// Clone `url` into `dest` (which must not exist yet; its parent must).
/// Returns the opened repository.
pub fn clone_repo(url: &str, dest: &Path) -> Result<RepoInfo> {
    if dest.exists() {
        return Err(GitError {
            message: format!("destination already exists: {}", dest.display()),
            command: String::new(),
            exit_code: None,
        });
    }
    let parent = dest.parent().filter(|p| p.is_dir()).ok_or_else(|| GitError {
        message: format!("parent folder does not exist: {}", dest.display()),
        command: String::new(),
        exit_code: None,
    })?;
    let dest_str = dest.to_string_lossy();
    run_git(parent, &["clone", "--", url, &dest_str])?;
    open_repo(dest)
}

/// Create a brand-new repository at `path` (created if missing) with an
/// initial branch named main.
pub fn init_repo(path: &Path) -> Result<RepoInfo> {
    std::fs::create_dir_all(path).map_err(|e| GitError {
        message: format!("could not create {}: {e}", path.display()),
        command: String::new(),
        exit_code: None,
    })?;
    run_git(path, &["init", "--initial-branch=main"])?;
    open_repo(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;

    #[test]
    fn opens_a_repo_and_reports_branch() {
        let repo = TestRepo::with_initial_commit();
        let info = open_repo(repo.path()).unwrap();
        assert_eq!(info.head, "main");
        assert!(!info.is_detached);
        assert_eq!(
            info.path,
            repo.path().canonicalize().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn opens_a_fresh_repo_with_no_commits() {
        let repo = TestRepo::empty();
        let info = open_repo(repo.path()).unwrap();
        assert_eq!(info.head, "main");
        assert!(!info.is_detached);
    }

    #[test]
    fn reports_detached_head() {
        let repo = TestRepo::with_initial_commit();
        repo.git(&["checkout", "--detach"]);
        let info = open_repo(repo.path()).unwrap();
        assert!(info.is_detached);
        assert!(!info.head.is_empty());
    }

    #[test]
    fn rejects_a_non_repo_directory() {
        let dir = tempfile::tempdir().unwrap();
        let err = open_repo(dir.path()).unwrap_err();
        assert!(err.message.contains("not a git repository"), "{err:?}");
    }

    #[test]
    fn rejects_a_missing_path() {
        let err = open_repo(std::path::Path::new("/nonexistent/nowhere")).unwrap_err();
        assert!(err.message.contains("not a directory"), "{err:?}");
    }

    #[test]
    fn clone_copies_a_repo_and_opens_it() {
        let origin = TestRepo::with_initial_commit();
        let bare = origin.add_bare_origin();
        origin.git(&["push", "origin", "main"]);

        let dest_parent = tempfile::tempdir().unwrap();
        let dest = dest_parent.path().join("cloned");
        let url = bare.path().to_string_lossy().to_string();

        let info = clone_repo(&url, &dest).unwrap();
        assert_eq!(info.name, "cloned");
        assert_eq!(info.head, "main");
        assert!(dest.join("README.md").exists());
    }

    #[test]
    fn clone_refuses_an_existing_destination() {
        let origin = TestRepo::with_initial_commit();
        let dest = tempfile::tempdir().unwrap();
        let url = origin.path().to_string_lossy().to_string();
        let err = clone_repo(&url, dest.path()).unwrap_err();
        assert!(err.message.contains("already exists"), "{err:?}");
    }

    #[test]
    fn init_creates_a_fresh_repo_on_main() {
        let parent = tempfile::tempdir().unwrap();
        let target = parent.path().join("newborn");
        let info = init_repo(&target).unwrap();
        assert_eq!(info.name, "newborn");
        assert_eq!(info.head, "main");
        assert!(target.join(".git").is_dir());
    }
}
