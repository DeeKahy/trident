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

        let commits = log(repo.path(), 10, 0).unwrap();
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

        let commits = log(repo.path(), 1, 0).unwrap();
        assert_eq!(commits[0].subject, "subject line");
    }

    #[test]
    fn amend_rewrites_head_without_adding_a_commit() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stage_file(repo.path(), "a.txt").unwrap();
        commit(repo.path(), "wrong subject", false).unwrap();

        commit(repo.path(), "right subject", true).unwrap();

        let commits = log(repo.path(), 10, 0).unwrap();
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
    fn commit_works_on_unborn_branch() {
        let repo = TestRepo::empty();
        repo.write("first.txt", "x\n");
        stage_file(repo.path(), "first.txt").unwrap();

        commit(repo.path(), "first commit", false).unwrap();

        let commits = log(repo.path(), 10, 0).unwrap();
        assert_eq!(commits.len(), 1);
    }
}
