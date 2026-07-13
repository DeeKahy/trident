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

/// A context count large enough that git prints every unchanged line of the
/// file around the hunks, i.e. the whole file with the changes highlighted.
/// Passed as `-U<n>` when the caller asks for the "full file" view.
const FULL_CONTEXT: &str = "-U100000";

/// Unified diff for a single file in the given mode. With `full`, the entire
/// file is shown (changes in context) rather than just the changed hunks.
pub fn diff_file(repo: &Path, path: &str, mode: DiffMode, full: bool) -> Result<String> {
    let mut args: Vec<&str> = vec!["diff", "--no-color"];
    if full {
        args.push(FULL_CONTEXT);
    }
    match mode {
        DiffMode::Worktree => args.extend_from_slice(&["--", path]),
        DiffMode::Staged => args.extend_from_slice(&["--cached", "--", path]),
        // git special-cases the literal path "/dev/null" in --no-index mode
        // (even on Windows), yielding an all-additions diff for a new file.
        DiffMode::Untracked => args.extend_from_slice(&["--no-index", "/dev/null", path]),
    }
    // `git diff` exits 1 when differences exist under --no-index; accept it.
    run_git_with_ok_codes(repo, &args, &[0, 1])
}

/// Full patch for a commit (what `git show` prints, without the header). With
/// `full`, every file is shown in its entirety rather than just the hunks.
pub fn commit_diff(repo: &Path, hash: &str, full: bool) -> Result<String> {
    let mut args: Vec<&str> = vec!["show", "--no-color", "--format=", "--patch"];
    if full {
        args.push(FULL_CONTEXT);
    }
    args.push(hash);
    run_git(repo, &args)
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
        let d = diff_file(repo.path(), "README.md", DiffMode::Worktree, false).unwrap();
        assert!(d.contains("+new line"), "{d}");
        assert!(d.contains("--- a/README.md"), "{d}");
    }

    #[test]
    fn staged_diff_is_separate_from_worktree() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# staged\n");
        repo.git(&["add", "README.md"]);
        repo.write("README.md", "# worktree\n");

        let staged = diff_file(repo.path(), "README.md", DiffMode::Staged, false).unwrap();
        assert!(staged.contains("+# staged"), "{staged}");
        assert!(!staged.contains("+# worktree"), "{staged}");

        let worktree = diff_file(repo.path(), "README.md", DiffMode::Worktree, false).unwrap();
        assert!(worktree.contains("+# worktree"), "{worktree}");
    }

    #[test]
    fn untracked_diff_shows_all_lines_as_additions() {
        let repo = TestRepo::with_initial_commit();
        repo.write("new.txt", "alpha\nbeta\n");
        let d = diff_file(repo.path(), "new.txt", DiffMode::Untracked, false).unwrap();
        assert!(d.contains("+alpha"), "{d}");
        assert!(d.contains("+beta"), "{d}");
    }

    #[test]
    fn unchanged_file_diffs_to_empty() {
        let repo = TestRepo::with_initial_commit();
        let d = diff_file(repo.path(), "README.md", DiffMode::Worktree, false).unwrap();
        assert!(d.is_empty());
    }

    #[test]
    fn full_context_diff_includes_unchanged_lines() {
        let repo = TestRepo::with_initial_commit();
        // A file whose changed line sits far from surrounding context lines that
        // a normal 3-line diff would omit but a full-file diff must include.
        repo.write("f.txt", "keep-top\na\nb\nc\nd\ne\nf\ng\nkeep-bottom\n");
        repo.git(&["add", "f.txt"]);
        repo.commit("add f.txt");
        repo.write("f.txt", "keep-top\na\nb\nc\nCHANGED\ne\nf\ng\nkeep-bottom\n");

        let normal = diff_file(repo.path(), "f.txt", DiffMode::Worktree, false).unwrap();
        assert!(normal.contains("+CHANGED"), "{normal}");
        // keep-bottom sits well below the change; a trimmed-context diff omits
        // it. (keep-top can't be used here: git echoes the line before the hunk
        // as its @@ section header, so it appears even when context is trimmed.)
        assert!(
            !normal.contains("keep-bottom"),
            "normal diff should trim distant context: {normal}"
        );

        let full = diff_file(repo.path(), "f.txt", DiffMode::Worktree, true).unwrap();
        assert!(full.contains("+CHANGED"), "{full}");
        assert!(full.contains("keep-top"), "full diff should show the whole file: {full}");
        assert!(full.contains("keep-bottom"), "{full}");
    }

    #[test]
    fn commit_diff_shows_the_patch() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "hello\n");
        repo.git(&["add", "a.txt"]);
        repo.commit("add a.txt");
        let head = repo.git(&["rev-parse", "HEAD"]);
        let d = commit_diff(repo.path(), head.trim(), false).unwrap();
        assert!(d.contains("+hello"), "{d}");
        assert!(d.contains("a.txt"), "{d}");
    }

    #[test]
    fn commit_diff_on_bad_hash_errors() {
        let repo = TestRepo::with_initial_commit();
        assert!(commit_diff(repo.path(), "deadbeef", false).is_err());
    }
}
