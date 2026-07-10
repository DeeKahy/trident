//! Staging, unstaging, and discarding changes. These are the first
//! repository-mutating operations, so every function's test proves it
//! touches only the path it was given.

use std::path::Path;

use super::{run_git, GitError, Result};

/// Stage a single file (tracked or untracked).
pub fn stage_file(repo: &Path, path: &str) -> Result<()> {
    run_git(repo, &["add", "--", path]).map(|_| ())
}

/// Stage everything, including untracked files.
pub fn stage_all(repo: &Path) -> Result<()> {
    run_git(repo, &["add", "--all"]).map(|_| ())
}

/// Unstage a single file, leaving its work-tree contents alone.
pub fn unstage_file(repo: &Path, path: &str) -> Result<()> {
    // `git restore --staged` requires a HEAD commit; `git reset` handles the
    // unborn-branch case (fresh repo, first commit not made yet) as well.
    run_git(repo, &["reset", "-q", "--", path]).map(|_| ())
}

/// Unstage everything, leaving the work tree alone.
pub fn unstage_all(repo: &Path) -> Result<()> {
    run_git(repo, &["reset", "-q"]).map(|_| ())
}

/// Throw away unstaged changes to a single file.
///
/// Tracked file: restore its work-tree contents from the index.
/// Untracked file: delete it. Directories are refused.
pub fn discard_file(repo: &Path, path: &str, untracked: bool) -> Result<()> {
    if untracked {
        let full = repo.join(path);
        if full.is_dir() {
            return Err(GitError {
                message: format!("refusing to discard a directory: {path}"),
                command: String::new(),
                exit_code: None,
            });
        }
        std::fs::remove_file(&full).map_err(|e| GitError {
            message: format!("could not delete {path}: {e}"),
            command: String::new(),
            exit_code: None,
        })
    } else {
        run_git(repo, &["restore", "--worktree", "--", path]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::status::status;
    use crate::git::testutil::TestRepo;

    #[test]
    fn stage_file_stages_only_that_file() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        repo.write("b.txt", "b\n");

        stage_file(repo.path(), "a.txt").unwrap();

        let s = status(repo.path()).unwrap();
        assert_eq!(s.staged.len(), 1);
        assert_eq!(s.staged[0].path, "a.txt");
        assert_eq!(s.untracked, vec!["b.txt"]);
    }

    #[test]
    fn unstage_file_keeps_worktree_contents() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# edited\n");
        stage_file(repo.path(), "README.md").unwrap();

        unstage_file(repo.path(), "README.md").unwrap();

        let s = status(repo.path()).unwrap();
        assert!(s.staged.is_empty());
        assert_eq!(s.unstaged.len(), 1);
        let contents = std::fs::read_to_string(repo.path().join("README.md")).unwrap();
        assert_eq!(contents, "# edited\n");
    }

    #[test]
    fn unstage_works_on_unborn_branch() {
        let repo = TestRepo::empty();
        repo.write("first.txt", "x\n");
        stage_file(repo.path(), "first.txt").unwrap();

        unstage_file(repo.path(), "first.txt").unwrap();

        let s = status(repo.path()).unwrap();
        assert!(s.staged.is_empty());
        assert_eq!(s.untracked, vec!["first.txt"]);
    }

    #[test]
    fn stage_all_and_unstage_all() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        repo.write("README.md", "# edited\n");

        stage_all(repo.path()).unwrap();
        let s = status(repo.path()).unwrap();
        assert_eq!(s.staged.len(), 2);
        assert!(s.untracked.is_empty());

        unstage_all(repo.path()).unwrap();
        let s = status(repo.path()).unwrap();
        assert!(s.staged.is_empty());
        assert_eq!(s.unstaged.len(), 1);
        assert_eq!(s.untracked, vec!["a.txt"]);
    }

    #[test]
    fn discard_tracked_restores_from_index_and_touches_nothing_else() {
        let repo = TestRepo::with_initial_commit();
        repo.write("other.txt", "keep me\n");
        repo.git(&["add", "other.txt"]);
        repo.commit("add other.txt");

        repo.write("README.md", "# ruined\n");
        repo.write("other.txt", "keep me edited\n");

        discard_file(repo.path(), "README.md", false).unwrap();

        let readme = std::fs::read_to_string(repo.path().join("README.md")).unwrap();
        assert_eq!(readme, "# test repo\n");
        // The other modified file is untouched.
        let other = std::fs::read_to_string(repo.path().join("other.txt")).unwrap();
        assert_eq!(other, "keep me edited\n");
    }

    #[test]
    fn discard_untracked_deletes_only_that_file() {
        let repo = TestRepo::with_initial_commit();
        repo.write("junk.txt", "x\n");
        repo.write("precious.txt", "y\n");

        discard_file(repo.path(), "junk.txt", true).unwrap();

        assert!(!repo.path().join("junk.txt").exists());
        assert!(repo.path().join("precious.txt").exists());
    }

    #[test]
    fn discard_refuses_a_directory() {
        let repo = TestRepo::with_initial_commit();
        repo.write("dir/file.txt", "x\n");
        let err = discard_file(repo.path(), "dir", true).unwrap_err();
        assert!(err.message.contains("refusing"), "{err:?}");
        assert!(repo.path().join("dir/file.txt").exists());
    }
}
