//! Stash: park all work-in-progress safely. Apply/pop/drop come with the
//! full stash UI later; for now the workspace only saves and lists.

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
}
