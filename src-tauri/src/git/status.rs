//! `git status` parsed from `--porcelain=v2 -z` output.
//!
//! Record formats (see git-status(1), "Porcelain Format Version 2"):
//!   `# <key> <value>`                                — branch headers
//!   `1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>`   — ordinary change
//!   `2 <XY> ... <Xscore> <path>` + NUL + `<origPath>` — rename or copy
//!   `u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>` — unmerged
//!   `? <path>` untracked, `! <path>` ignored
//! In XY, X is the staged (index) status and Y the work-tree status;
//! `.` means unchanged on that side.

use std::path::Path;

use super::types::{BranchStatus, ChangeKind, FileChange, Status};
use super::{run_git, Result};

pub fn status(repo: &Path) -> Result<Status> {
    let raw = run_git(
        repo,
        &[
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
        ],
    )?;
    Ok(parse(&raw))
}

fn parse(raw: &str) -> Status {
    let mut result = Status {
        branch: BranchStatus::default(),
        staged: Vec::new(),
        unstaged: Vec::new(),
        untracked: Vec::new(),
        conflicted: Vec::new(),
    };

    let mut tokens = raw.split('\0').peekable();
    while let Some(record) = tokens.next() {
        if record.is_empty() {
            continue;
        }
        match record.as_bytes()[0] {
            b'#' => parse_header(record, &mut result.branch),
            b'1' => {
                if let Some((xy, path)) = split_fields(record, 8) {
                    push_change(&mut result, xy, path, None);
                }
            }
            b'2' => {
                // The rename/copy source arrives as the next NUL token.
                let orig = tokens.next().map(str::to_string);
                if let Some((xy, path)) = split_fields(record, 9) {
                    push_change(&mut result, xy, path, orig);
                }
            }
            b'u' => {
                if let Some((_, path)) = split_fields(record, 10) {
                    result.conflicted.push(path.to_string());
                }
            }
            b'?' => result.untracked.push(record[2..].to_string()),
            b'!' => {} // ignored files: not shown
            _ => {}
        }
    }

    result
}

/// Split a record into `n_fields` space-separated fields plus the path
/// remainder (paths may contain spaces). Returns the XY field and the path.
fn split_fields(record: &str, n_fields: usize) -> Option<(&str, &str)> {
    let mut parts = record.splitn(n_fields + 1, ' ');
    parts.next()?; // record type tag
    let xy = parts.next()?;
    let path = parts.last()?;
    Some((xy, path))
}

fn push_change(result: &mut Status, xy: &str, path: &str, orig_path: Option<String>) {
    let mut chars = xy.chars();
    let x = chars.next().unwrap_or('.');
    let y = chars.next().unwrap_or('.');

    if let Some(kind) = ChangeKind::from_letter(x) {
        result.staged.push(FileChange {
            path: path.to_string(),
            orig_path: orig_path.clone(),
            kind,
        });
    }
    if let Some(kind) = ChangeKind::from_letter(y) {
        result.unstaged.push(FileChange {
            path: path.to_string(),
            orig_path,
            kind,
        });
    }
}

fn parse_header(record: &str, branch: &mut BranchStatus) {
    let Some(rest) = record.strip_prefix("# ") else {
        return;
    };
    let Some((key, value)) = rest.split_once(' ') else {
        return;
    };
    match key {
        "branch.head" => branch.head = value.to_string(),
        "branch.oid" => {
            branch.oid = if value == "(initial)" {
                String::new()
            } else {
                value.chars().take(8).collect()
            };
        }
        "branch.upstream" => branch.upstream = Some(value.to_string()),
        "branch.ab" => {
            for part in value.split(' ') {
                if let Some(n) = part.strip_prefix('+') {
                    branch.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    branch.behind = n.parse().unwrap_or(0);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;
    use crate::git::types::ChangeKind;

    #[test]
    fn clean_repo_has_empty_lists() {
        let repo = TestRepo::with_initial_commit();
        let s = status(repo.path()).unwrap();
        assert_eq!(s.branch.head, "main");
        assert!(!s.branch.oid.is_empty());
        assert!(s.staged.is_empty());
        assert!(s.unstaged.is_empty());
        assert!(s.untracked.is_empty());
        assert!(s.conflicted.is_empty());
    }

    #[test]
    fn untracked_file_is_listed() {
        let repo = TestRepo::with_initial_commit();
        repo.write("new.txt", "hello\n");
        let s = status(repo.path()).unwrap();
        assert_eq!(s.untracked, vec!["new.txt"]);
        assert!(s.staged.is_empty());
    }

    #[test]
    fn staged_new_file_is_added() {
        let repo = TestRepo::with_initial_commit();
        repo.write("new.txt", "hello\n");
        repo.git(&["add", "new.txt"]);
        let s = status(repo.path()).unwrap();
        assert_eq!(s.staged.len(), 1);
        assert_eq!(s.staged[0].path, "new.txt");
        assert_eq!(s.staged[0].kind, ChangeKind::Added);
        assert!(s.untracked.is_empty());
    }

    #[test]
    fn modified_file_is_unstaged() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# changed\n");
        let s = status(repo.path()).unwrap();
        assert_eq!(s.unstaged.len(), 1);
        assert_eq!(s.unstaged[0].path, "README.md");
        assert_eq!(s.unstaged[0].kind, ChangeKind::Modified);
    }

    #[test]
    fn file_can_be_both_staged_and_unstaged() {
        let repo = TestRepo::with_initial_commit();
        repo.write("README.md", "# staged version\n");
        repo.git(&["add", "README.md"]);
        repo.write("README.md", "# worktree version\n");
        let s = status(repo.path()).unwrap();
        assert_eq!(s.staged.len(), 1);
        assert_eq!(s.unstaged.len(), 1);
        assert_eq!(s.staged[0].path, "README.md");
        assert_eq!(s.unstaged[0].path, "README.md");
    }

    #[test]
    fn rename_reports_original_path() {
        let repo = TestRepo::with_initial_commit();
        repo.git(&["mv", "README.md", "RENAMED.md"]);
        let s = status(repo.path()).unwrap();
        assert_eq!(s.staged.len(), 1);
        assert_eq!(s.staged[0].kind, ChangeKind::Renamed);
        assert_eq!(s.staged[0].path, "RENAMED.md");
        assert_eq!(s.staged[0].orig_path.as_deref(), Some("README.md"));
    }

    #[test]
    fn path_with_spaces_survives_parsing() {
        let repo = TestRepo::with_initial_commit();
        repo.write("has spaces in name.txt", "x\n");
        repo.git(&["add", "has spaces in name.txt"]);
        let s = status(repo.path()).unwrap();
        assert_eq!(s.staged[0].path, "has spaces in name.txt");
    }

    #[test]
    fn ahead_count_against_upstream() {
        let repo = TestRepo::with_initial_commit();
        repo.git(&["branch", "base"]);
        repo.write("more.txt", "x\n");
        repo.git(&["add", "more.txt"]);
        repo.commit("second commit");
        repo.git(&["branch", "--set-upstream-to=base"]);
        let s = status(repo.path()).unwrap();
        assert_eq!(s.branch.upstream.as_deref(), Some("base"));
        assert_eq!(s.branch.ahead, 1);
        assert_eq!(s.branch.behind, 0);
    }

    #[test]
    fn merge_conflict_is_reported() {
        let repo = TestRepo::with_initial_commit();
        repo.git(&["checkout", "-b", "feature"]);
        repo.write("README.md", "# feature version\n");
        repo.git(&["add", "README.md"]);
        repo.commit("feature change");
        repo.git(&["checkout", "main"]);
        repo.write("README.md", "# main version\n");
        repo.git(&["add", "README.md"]);
        repo.commit("main change");
        repo.git_ignore_status(&["merge", "feature"]);
        let s = status(repo.path()).unwrap();
        assert_eq!(s.conflicted, vec!["README.md"]);
    }

    #[test]
    fn fresh_repo_reports_unborn_branch() {
        let repo = TestRepo::empty();
        let s = status(repo.path()).unwrap();
        assert_eq!(s.branch.head, "main");
        assert_eq!(s.branch.oid, "");
    }
}
