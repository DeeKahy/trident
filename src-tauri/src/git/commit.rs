//! Creating commits.

use std::path::Path;

use super::{run_git, GitError, Result};

/// Commit whatever is staged. Returns the new commit's short hash.
/// `amend` rewrites the current HEAD commit instead of creating a new one.
pub fn commit(repo: &Path, message: &str, amend: bool) -> Result<String> {
    if message.trim().is_empty() {
        return Err(GitError {
            message: "commit message is empty".to_string(),
            command: String::new(),
            exit_code: None,
        });
    }
    let mut args = vec!["commit", "-m", message];
    if amend {
        args.push("--amend");
    }
    run_git(repo, &args)?;
    let hash = run_git(repo, &["rev-parse", "--short", "HEAD"])?;
    Ok(hash.trim().to_string())
}

/// Rewrite only the message of the HEAD commit, leaving the index alone
/// (`--only` with no paths amends without sweeping in staged changes).
/// The UI offers this only for commits that haven't been pushed.
pub fn reword_head(repo: &Path, message: &str) -> Result<String> {
    if message.trim().is_empty() {
        return Err(GitError {
            message: "commit message is empty".to_string(),
            command: String::new(),
            exit_code: None,
        });
    }
    run_git(repo, &["commit", "--amend", "--only", "-m", message])?;
    let hash = run_git(repo, &["rev-parse", "--short", "HEAD"])?;
    Ok(hash.trim().to_string())
}

/// Undo the HEAD commit. `keep_changes` leaves its files staged (soft
/// reset); otherwise the work is discarded entirely (hard reset).
pub fn undo_last_commit(repo: &Path, keep_changes: bool) -> Result<()> {
    let mode = if keep_changes { "--soft" } else { "--hard" };
    run_git(repo, &["reset", mode, "HEAD~1"]).map(|_| ())
}

/// Revert a commit by adding an inverse commit: safe on shared history.
/// A conflicting revert is aborted so the repo is left untouched.
pub fn revert_commit(repo: &Path, hash: &str) -> Result<()> {
    match run_git(repo, &["revert", "--no-edit", hash]) {
        Ok(_) => Ok(()),
        Err(mut e) => {
            let _ = run_git(repo, &["revert", "--abort"]);
            e.message = format!("{} (revert aborted, nothing was changed)", e.message);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::log::log;
    use crate::git::stage::stage_file;
    use crate::git::status::status;
    use crate::git::testutil::TestRepo;

    #[test]
    fn commit_creates_a_commit_and_cleans_status() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();

        let hash = commit(repo.path(), "add a.txt", false).unwrap();
        assert!(!hash.is_empty());

        let commits = log(repo.path(), 10, 0, false).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "add a.txt");
        assert_eq!(commits[0].short_hash, hash);

        let s = status(repo.path()).unwrap();
        assert!(s.staged.is_empty());
        assert!(s.unstaged.is_empty());
    }

    #[test]
    fn commit_supports_multi_line_messages() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();

        commit(repo.path(), "subject line\n\nbody paragraph", false).unwrap();

        let commits = log(repo.path(), 1, 0, false).unwrap();
        assert_eq!(commits[0].subject, "subject line");
    }

    #[test]
    fn amend_rewrites_head_without_adding_a_commit() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();
        commit(repo.path(), "wrong subject", false).unwrap();

        commit(repo.path(), "right subject", true).unwrap();

        let commits = log(repo.path(), 10, 0, false).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "right subject");
    }

    #[test]
    fn empty_message_is_rejected_before_reaching_git() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();
        let err = commit(repo.path(), "   ", false).unwrap_err();
        assert!(err.message.contains("empty"), "{err:?}");
    }

    #[test]
    fn nothing_staged_surfaces_gits_error() {
        let repo = TestRepo::with_initial_commit();
        assert!(commit(repo.path(), "no changes", false).is_err());
    }

    #[test]
    fn reword_changes_message_but_not_the_index() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();

        reword_head(repo.path(), "better subject").unwrap();

        let commits = log(repo.path(), 10, 0, false).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].subject, "better subject");
        // The staged file was not swept into the amended commit.
        let s = status(repo.path()).unwrap();
        assert_eq!(s.staged.len(), 1);
        assert_eq!(s.staged[0].path, "a.txt");
    }

    #[test]
    fn undo_keeping_changes_leaves_files_staged() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();
        commit(repo.path(), "oops", false).unwrap();

        undo_last_commit(repo.path(), true).unwrap();

        let commits = log(repo.path(), 10, 0, false).unwrap();
        assert_eq!(commits.len(), 1);
        let s = status(repo.path()).unwrap();
        assert_eq!(s.staged.len(), 1);
        assert!(repo.path().join("a.txt").exists());
    }

    #[test]
    fn undo_discarding_removes_the_work() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();
        commit(repo.path(), "oops", false).unwrap();

        undo_last_commit(repo.path(), false).unwrap();

        let commits = log(repo.path(), 10, 0, false).unwrap();
        assert_eq!(commits.len(), 1);
        assert!(!repo.path().join("a.txt").exists());
    }

    #[test]
    fn undo_on_root_commit_errors() {
        let repo = TestRepo::with_initial_commit();
        assert!(undo_last_commit(repo.path(), true).is_err());
    }

    #[test]
    fn revert_adds_an_inverse_commit() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();
        commit(repo.path(), "add a.txt", false).unwrap();
        let head = repo.git(&["rev-parse", "HEAD"]);

        revert_commit(repo.path(), head.trim()).unwrap();

        let commits = log(repo.path(), 10, 0, false).unwrap();
        assert_eq!(commits.len(), 3);
        assert!(commits[0].subject.contains("Revert"));
        assert!(!repo.path().join("a.txt").exists());
    }

    #[test]
    fn commit_works_on_unborn_branch() {
        let repo = TestRepo::empty();
        repo.write("first.txt", "x\n");
        stage_file(repo.path(), "first.txt").unwrap();

        commit(repo.path(), "first commit", false).unwrap();

        let commits = log(repo.path(), 10, 0, false).unwrap();
        assert_eq!(commits.len(), 1);
    }
}
