//! Fetch, pull, and push. Authentication rides on git's own credential
//! helpers and the user's SSH agent; nothing here handles secrets.

use std::path::Path;

use super::{run_git, Result};

/// Fetch all remotes, pruning refs deleted upstream.
pub fn fetch(repo: &Path) -> Result<()> {
    run_git(repo, &["fetch", "--all", "--prune"]).map(|_| ())
}

/// Pull the current branch, fast-forward only. A diverged branch is an
/// error rather than a surprise merge commit; the guided flow decides what
/// to do about divergence (Phase 4 territory).
pub fn pull(repo: &Path) -> Result<()> {
    run_git(repo, &["pull", "--ff-only"]).map(|_| ())
}

/// Push the current branch. When it has no upstream yet, retry once with
/// `--set-upstream origin HEAD` so a fresh branch pushes without ceremony.
pub fn push(repo: &Path) -> Result<()> {
    match run_git(repo, &["push"]) {
        Ok(_) => Ok(()),
        Err(e) if e.message.contains("no upstream branch") => {
            run_git(repo, &["push", "--set-upstream", "origin", "HEAD"]).map(|_| ())
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::branch::branches;
    use crate::git::log::log;
    use crate::git::testutil::TestRepo;

    #[test]
    fn push_sets_upstream_on_first_push() {
        let repo = TestRepo::with_initial_commit();
        let _bare = repo.add_bare_origin();

        push(repo.path()).unwrap();

        let all = branches(repo.path()).unwrap();
        let main = all.iter().find(|b| b.name == "main" && !b.is_remote).unwrap();
        assert_eq!(main.upstream.as_deref(), Some("origin/main"));
        assert!(all.iter().any(|b| b.name == "origin/main" && b.is_remote));
    }

    #[test]
    fn push_then_fetch_round_trips_between_clones() {
        // First clone pushes a commit; second clone pulls it.
        let writer = TestRepo::with_initial_commit();
        let bare = writer.add_bare_origin();
        push(writer.path()).unwrap();

        let reader = TestRepo::empty();
        let url = bare.path().to_str().unwrap().to_string();
        reader.git(&["remote", "add", "origin", &url]);
        reader.git(&["pull", "origin", "main"]);
        reader.git(&["branch", "--set-upstream-to=origin/main"]);
        assert_eq!(log(reader.path(), 10, 0).unwrap().len(), 1);

        writer.write("news.txt", "fresh\n");
        writer.git(&["add", "news.txt"]);
        writer.commit("news");
        push(writer.path()).unwrap();

        pull(reader.path()).unwrap();
        let commits = log(reader.path(), 10, 0).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "news");
    }

    #[test]
    fn fetch_updates_remote_refs_without_touching_worktree() {
        let writer = TestRepo::with_initial_commit();
        let bare = writer.add_bare_origin();
        push(writer.path()).unwrap();

        let reader = TestRepo::empty();
        let url = bare.path().to_str().unwrap().to_string();
        reader.git(&["remote", "add", "origin", &url]);
        reader.git(&["pull", "origin", "main"]);
        reader.git(&["branch", "--set-upstream-to=origin/main"]);

        writer.write("news.txt", "fresh\n");
        writer.git(&["add", "news.txt"]);
        writer.commit("news");
        push(writer.path()).unwrap();

        fetch(reader.path()).unwrap();

        // Local branch is now behind its upstream, but the worktree is unchanged.
        let all = branches(reader.path()).unwrap();
        let main = all.iter().find(|b| b.name == "main" && !b.is_remote).unwrap();
        assert_eq!(main.behind, 1);
        assert!(!reader.path().join("news.txt").exists());
    }

    #[test]
    fn pull_refuses_diverged_branches() {
        let writer = TestRepo::with_initial_commit();
        let bare = writer.add_bare_origin();
        push(writer.path()).unwrap();

        let reader = TestRepo::empty();
        let url = bare.path().to_str().unwrap().to_string();
        reader.git(&["remote", "add", "origin", &url]);
        reader.git(&["pull", "origin", "main"]);
        reader.git(&["branch", "--set-upstream-to=origin/main"]);

        // Both sides commit: divergence.
        writer.write("theirs.txt", "x\n");
        writer.git(&["add", "theirs.txt"]);
        writer.commit("their commit");
        push(writer.path()).unwrap();

        reader.write("ours.txt", "y\n");
        reader.git(&["add", "ours.txt"]);
        reader.commit("our commit");

        assert!(pull(reader.path()).is_err());
        // Nothing was merged or lost.
        let commits = log(reader.path(), 10, 0).unwrap();
        assert_eq!(commits[0].subject, "our commit");
    }

    #[test]
    fn push_without_remote_errors() {
        let repo = TestRepo::with_initial_commit();
        assert!(push(repo.path()).is_err());
    }
}
