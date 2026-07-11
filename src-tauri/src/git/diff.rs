//! Diffs as raw unified-diff text. The frontend renders the text directly,
//! so these functions stay string-in/string-out; structured diff parsing
//! can come later when the UI needs side-by-side or hunk staging.

use std::path::Path;

use serde::Deserialize;

use super::{run_git, run_git_with_ok_codes, Result};

/// Which version of a file to diff. Serialized lowercase to match the
/// string the frontend sends (`"worktree" | "staged" | "untracked"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffMode {
    /// Unstaged changes: work tree vs index.
    Worktree,
    /// Staged changes: index vs HEAD.
    Staged,
    /// A file git doesn't know yet, shown as an all-additions diff.
    Untracked,
}

/// Unified diff for a single file in the given mode.
pub fn diff_file(repo: &Path, path: &str, mode: DiffMode) -> Result<String> {
    // `git diff` exits 1 when differences exist under --no-index; accept it.
    match mode {
        DiffMode::Worktree => {
            run_git_with_ok_codes(repo, &["diff", "--no-color", "--", path], &[0, 1])
        }
        DiffMode::Staged => {
            run_git_with_ok_codes(repo, &["diff", "--no-color", "--cached", "--", path], &[0, 1])
        }
        // git special-cases the literal path "/dev/null" in --no-index mode
        // (even on Windows), yielding an all-additions diff for a new file.
        DiffMode::Untracked => run_git_with_ok_codes(
            repo,
            &["diff", "--no-color", "--no-index", "/dev/null", path],
            &[0, 1],
        ),
    }
}

/// Full patch for a commit (what `git show` prints, without the header).
pub fn commit_diff(repo: &Path, hash: &str) -> Result<String> {
    run_git(
        repo,
        &["show", "--no-color", "--format=", "--patch", hash],
    )
}

/// Parse `--numstat -z` output into path -> (additions, deletions).
///
/// Each NUL token is `ADD\tDEL\tpath`; renames leave the path empty and
/// send old and new paths as the following two tokens (keyed by new path).
/// Binary files report `-` and count as 0.
pub fn parse_numstat(raw: &str) -> std::collections::HashMap<String, (u32, u32)> {
    let mut map = std::collections::HashMap::new();
    let mut tokens = raw.split('\0');
    while let Some(token) = tokens.next() {
        if token.is_empty() {
            continue;
        }
        let mut cols = token.splitn(3, '\t');
        let add = cols.next().and_then(|c| c.parse().ok()).unwrap_or(0);
        let del = cols.next().and_then(|c| c.parse().ok()).unwrap_or(0);
        let Some(path) = cols.next() else { continue };
        let key = if path.is_empty() {
            // Rename: consume the old path, key by the new one.
            let _old = tokens.next();
            match tokens.next() {
                Some(new_path) => new_path.to_string(),
                None => continue,
            }
        } else {
            path.to_string()
        };
        map.insert(key, (add, del));
    }
    map
}

/// Numstat for work-tree changes (staged: index vs HEAD, otherwise
/// work tree vs index).
pub fn numstat(repo: &Path, staged: bool) -> Result<std::collections::HashMap<String, (u32, u32)>> {
    let raw = if staged {
        run_git_with_ok_codes(repo, &["diff", "--numstat", "-z", "--cached"], &[0, 1])?
    } else {
        run_git_with_ok_codes(repo, &["diff", "--numstat", "-z"], &[0, 1])?
    };
    Ok(parse_numstat(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;

    #[test]
    fn worktree_diff_shows_the_edit() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# test repo\nnew line\n");
        let d = diff_file(repo.path(), "README.md", DiffMode::Worktree).unwrap();
        assert!(d.contains("+new line"), "{d}");
        assert!(d.contains("--- a/README.md"), "{d}");
    }

    #[test]
    fn staged_diff_is_separate_from_worktree() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# staged\n");
        repo.git(&["add", "README.md"]);
        repo.write("README.md", "# worktree\n");

        let staged = diff_file(repo.path(), "README.md", DiffMode::Staged).unwrap();
        assert!(staged.contains("+# staged"), "{staged}");
        assert!(!staged.contains("+# worktree"), "{staged}");

        let worktree = diff_file(repo.path(), "README.md", DiffMode::Worktree).unwrap();
        assert!(worktree.contains("+# worktree"), "{worktree}");
    }

    #[test]
    fn untracked_diff_shows_all_lines_as_additions() {
        let repo = TestRepo::with_initial_commit();
        repo.write("new.txt", "alpha\nbeta\n");
        let d = diff_file(repo.path(), "new.txt", DiffMode::Untracked).unwrap();
        assert!(d.contains("+alpha"), "{d}");
        assert!(d.contains("+beta"), "{d}");
    }

    #[test]
    fn unchanged_file_diffs_to_empty() {
        let repo = TestRepo::with_initial_commit();
        let d = diff_file(repo.path(), "README.md", DiffMode::Worktree).unwrap();
        assert!(d.is_empty());
    }

    #[test]
    fn commit_diff_shows_the_patch() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "hello\n");
        repo.git(&["add", "a.txt"]);
        repo.commit("add a.txt");
        let head = repo.git(&["rev-parse", "HEAD"]);
        let d = commit_diff(repo.path(), head.trim()).unwrap();
        assert!(d.contains("+hello"), "{d}");
        assert!(d.contains("a.txt"), "{d}");
    }

    #[test]
    fn commit_diff_on_bad_hash_errors() {
        let repo = TestRepo::with_initial_commit();
        assert!(commit_diff(repo.path(), "deadbeef").is_err());
    }
}
