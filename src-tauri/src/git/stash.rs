//! Stash: park all work-in-progress safely, and get it back. Index 0 is the
//! most recent stash, matching the order `stash_list` returns.

use std::path::Path;

use super::{run_git, Result};

/// Stash messages, newest first. Empty when there are no stashes.
pub fn stash_list(repo: &Path) -> Result<Vec<String>> {
    let raw = run_git(repo, &["stash", "list", "--format=%gs"])?;
    Ok(raw.lines().map(str::to_string).collect())
}

/// Stash every change including untracked files. A clean tree is a no-op.
pub fn stash_all(repo: &Path, message: &str) -> Result<()> {
    if message.trim().is_empty() {
        run_git(repo, &["stash", "push", "--include-untracked"])?;
    } else {
        run_git(repo, &["stash", "push", "--include-untracked", "-m", message])?;
    }
    Ok(())
}

/// Restore a stash into the working tree and remove it from the stack
/// (`git stash pop`). On a conflict git keeps the stash, so nothing is lost.
pub fn stash_pop(repo: &Path, index: usize) -> Result<()> {
    run_git(repo, &["stash", "pop", &format!("stash@{{{index}}}")])?;
    Ok(())
}

/// Restore a stash into the working tree but keep it on the stack
/// (`git stash apply`), so the same work can be applied again elsewhere.
pub fn stash_apply(repo: &Path, index: usize) -> Result<()> {
    run_git(repo, &["stash", "apply", &format!("stash@{{{index}}}")])?;
    Ok(())
}

/// Delete a stash without restoring it (`git stash drop`).
pub fn stash_drop(repo: &Path, index: usize) -> Result<()> {
    run_git(repo, &["stash", "drop", &format!("stash@{{{index}}}")])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::status::status;
    use crate::git::testutil::TestRepo;

    #[test]
    fn stash_all_parks_tracked_and_untracked_work() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# dirty\n");
        repo.write("new.txt", "untracked\n");

        stash_all(repo.path(), "wip: everything").unwrap();

        let s = status(repo.path()).unwrap();
        assert!(s.staged.is_empty());
        assert!(s.unstaged.is_empty());
        assert!(s.untracked.is_empty());

        let list = stash_list(repo.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].contains("wip: everything"), "{list:?}");

        // The work is recoverable, not gone.
        repo.git(&["stash", "pop"]);
        let s = status(repo.path()).unwrap();
        assert_eq!(s.unstaged.len(), 1);
        assert_eq!(s.untracked, vec!["new.txt"]);
    }

    #[test]
    fn clean_tree_stash_is_a_no_op() {
        let repo = TestRepo::with_initial_commit();
        stash_all(repo.path(), "").unwrap();
        assert!(stash_list(repo.path()).unwrap().is_empty());
    }

    #[test]
    fn pop_restores_the_work_and_removes_the_stash() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# dirty\n");
        stash_all(repo.path(), "wip").unwrap();
        assert_eq!(stash_list(repo.path()).unwrap().len(), 1);

        stash_pop(repo.path(), 0).unwrap();
        let s = status(repo.path()).unwrap();
        assert_eq!(s.unstaged.len(), 1);
        assert!(stash_list(repo.path()).unwrap().is_empty());
    }

    #[test]
    fn apply_restores_the_work_but_keeps_the_stash() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# dirty\n");
        stash_all(repo.path(), "wip").unwrap();

        stash_apply(repo.path(), 0).unwrap();
        let s = status(repo.path()).unwrap();
        assert_eq!(s.unstaged.len(), 1);
        assert_eq!(stash_list(repo.path()).unwrap().len(), 1);
    }

    #[test]
    fn drop_deletes_the_stash_without_touching_the_tree() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# dirty\n");
        stash_all(repo.path(), "wip").unwrap();

        stash_drop(repo.path(), 0).unwrap();
        assert!(stash_list(repo.path()).unwrap().is_empty());
        // Nothing was restored, so the tree is clean again.
        let s = status(repo.path()).unwrap();
        assert!(s.unstaged.is_empty() && s.untracked.is_empty());
    }

    #[test]
    fn pop_targets_the_requested_index() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        stash_all(repo.path(), "first").unwrap();
        repo.write("b.txt", "b\n");
        stash_all(repo.path(), "second").unwrap();

        // stash@{0} is "second", stash@{1} is "first"; pop the older one.
        stash_pop(repo.path(), 1).unwrap();
        let s = status(repo.path()).unwrap();
        assert_eq!(s.untracked, vec!["a.txt"]);
        let list = stash_list(repo.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].contains("second"), "{list:?}");
    }
}
