//! Commit history via `git log` with a custom field-separated format.
//!
//! Fields are joined with the ASCII unit separator (0x1f) and records with
//! the record separator (0x1e), neither of which can appear in hashes,
//! names, or a single-line subject.

use std::path::Path;

use super::types::{ChangeKind, CommitDetails, CommitInfo, FileChange, Signature};
use super::{run_git, GitError, Result};

const FIELD_SEP: char = '\u{1f}';
const RECORD_SEP: char = '\u{1e}';

/// Return `limit` commits starting `skip` entries below HEAD, newest first.
/// An empty (unborn) repository yields an empty list rather than an error.
pub fn log(repo: &Path, limit: u32, skip: u32) -> Result<Vec<CommitInfo>> {
    let format = "%H\u{1f}%h\u{1f}%an\u{1f}%ae\u{1f}%aI\u{1f}%P\u{1f}%s\u{1e}";
    let limit_arg = format!("-n{limit}");
    let skip_arg = format!("--skip={skip}");
    let format_arg = format!("--format={format}");

    let raw = match run_git(repo, &["log", &limit_arg, &skip_arg, &format_arg]) {
        Ok(raw) => raw,
        // A repo with no commits has no HEAD to walk; that's an empty log.
        Err(e) if e.message.contains("does not have any commits") => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    Ok(raw
        .split(RECORD_SEP)
        .filter_map(|record| parse_record(record.trim_start_matches('\n')))
        .collect())
}

/// Full details for one commit: both signatures, the whole message, and the
/// list of files it changed.
pub fn commit_details(repo: &Path, hash: &str) -> Result<CommitDetails> {
    let format =
        "%H\u{1f}%h\u{1f}%an\u{1f}%ae\u{1f}%aI\u{1f}%cn\u{1f}%ce\u{1f}%cI\u{1f}%P\u{1f}%B";
    let format_arg = format!("--format={format}");
    let raw = run_git(repo, &["show", "--no-patch", &format_arg, hash])?;

    let mut fields = raw.split(FIELD_SEP);
    let mut next = |what: &str| {
        fields.next().map(str::to_string).ok_or_else(|| GitError {
            message: format!("unexpected git show output: missing {what}"),
            command: format!("git show {hash}"),
            exit_code: None,
        })
    };

    let full_hash = next("hash")?;
    let short_hash = next("short hash")?;
    let author = Signature {
        name: next("author name")?,
        email: next("author email")?,
        date: next("author date")?,
    };
    let committer = Signature {
        name: next("committer name")?,
        email: next("committer email")?,
        date: next("committer date")?,
    };
    let parents_raw = next("parents")?;
    let message = next("message")?.trim_end().to_string();

    let parents = if parents_raw.is_empty() {
        Vec::new()
    } else {
        parents_raw.split(' ').map(str::to_string).collect()
    };

    Ok(CommitDetails {
        hash: full_hash,
        short_hash,
        author,
        committer,
        message,
        parents,
        files: changed_files(repo, hash)?,
    })
}

/// Files changed by a commit, from `--name-status -z` output: a status token
/// (`M`, `A`, `D`, `T`, or `R<score>`/`C<score>` followed by two paths), each
/// NUL-terminated.
fn changed_files(repo: &Path, hash: &str) -> Result<Vec<FileChange>> {
    let raw = run_git(
        repo,
        &["show", "--format=", "--name-status", "-z", hash],
    )?;

    let mut files = Vec::new();
    let mut tokens = raw.split('\0');
    while let Some(status) = tokens.next() {
        if status.is_empty() {
            continue;
        }
        let letter = status.chars().next().unwrap_or('.');
        let Some(kind) = ChangeKind::from_letter(letter) else {
            continue;
        };
        let Some(first_path) = tokens.next() else {
            break;
        };
        let (path, orig_path) = if matches!(kind, ChangeKind::Renamed | ChangeKind::Copied) {
            match tokens.next() {
                Some(new_path) => (new_path.to_string(), Some(first_path.to_string())),
                None => break,
            }
        } else {
            (first_path.to_string(), None)
        };
        files.push(FileChange {
            path,
            orig_path,
            kind,
        });
    }
    Ok(files)
}

fn parse_record(record: &str) -> Option<CommitInfo> {
    if record.trim().is_empty() {
        return None;
    }
    let mut fields = record.split(FIELD_SEP);
    Some(CommitInfo {
        hash: fields.next()?.to_string(),
        short_hash: fields.next()?.to_string(),
        author: fields.next()?.to_string(),
        email: fields.next()?.to_string(),
        date: fields.next()?.to_string(),
        parents: {
            let parents = fields.next()?;
            if parents.is_empty() {
                Vec::new()
            } else {
                parents.split(' ').map(str::to_string).collect()
            }
        },
        subject: fields.next()?.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;

    #[test]
    fn returns_commits_newest_first() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        repo.git(&["add", "a.txt"]);
        repo.commit("second commit");

        let commits = log(repo.path(), 50, 0).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "second commit");
        assert_eq!(commits[1].subject, "initial commit");
        assert_eq!(commits[0].author, "Test User");
        assert_eq!(commits[0].email, "test@example.com");
        // The second commit's parent is the first commit.
        assert_eq!(commits[0].parents, vec![commits[1].hash.clone()]);
        assert!(commits[1].parents.is_empty());
    }

    #[test]
    fn respects_limit_and_skip() {
        let repo = TestRepo::with_initial_commit();
        for i in 0..5 {
            repo.write("counter.txt", &format!("{i}\n"));
            repo.git(&["add", "counter.txt"]);
            repo.commit(&format!("commit {i}"));
        }
        let commits = log(repo.path(), 2, 1).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "commit 3");
        assert_eq!(commits[1].subject, "commit 2");
    }

    #[test]
    fn empty_repo_yields_empty_log() {
        let repo = TestRepo::empty();
        let commits = log(repo.path(), 50, 0).unwrap();
        assert!(commits.is_empty());
    }

    #[test]
    fn details_include_signatures_message_and_files() {
        let repo = TestRepo::with_initial_commit();
        repo.write("a.txt", "a\n");
        repo.write("README.md", "# changed\n");
        repo.git(&["add", "."]);
        repo.commit("subject here\n\nbody first line\nbody second line");

        let head = repo.git(&["rev-parse", "HEAD"]);
        let d = commit_details(repo.path(), head.trim()).unwrap();

        assert_eq!(d.hash, head.trim());
        assert_eq!(d.author.name, "Test User");
        assert_eq!(d.author.email, "test@example.com");
        assert_eq!(d.committer.name, "Test User");
        assert!(d.author.date.starts_with("2026-01-02T"));
        assert_eq!(
            d.message,
            "subject here\n\nbody first line\nbody second line"
        );
        assert_eq!(d.parents.len(), 1);

        let mut paths: Vec<_> = d.files.iter().map(|f| f.path.as_str()).collect();
        paths.sort();
        assert_eq!(paths, vec!["README.md", "a.txt"]);
        let readme = d.files.iter().find(|f| f.path == "README.md").unwrap();
        assert_eq!(readme.kind, crate::git::types::ChangeKind::Modified);
        let a = d.files.iter().find(|f| f.path == "a.txt").unwrap();
        assert_eq!(a.kind, crate::git::types::ChangeKind::Added);
    }

    #[test]
    fn details_report_renames_with_orig_path() {
        let repo = TestRepo::with_initial_commit();
        repo.git(&["mv", "README.md", "MOVED.md"]);
        repo.commit("rename it");

        let head = repo.git(&["rev-parse", "HEAD"]);
        let d = commit_details(repo.path(), head.trim()).unwrap();

        assert_eq!(d.files.len(), 1);
        assert_eq!(d.files[0].kind, crate::git::types::ChangeKind::Renamed);
        assert_eq!(d.files[0].path, "MOVED.md");
        assert_eq!(d.files[0].orig_path.as_deref(), Some("README.md"));
    }

    #[test]
    fn details_of_root_commit_have_no_parents() {
        let repo = TestRepo::with_initial_commit();
        let head = repo.git(&["rev-parse", "HEAD"]);
        let d = commit_details(repo.path(), head.trim()).unwrap();
        assert!(d.parents.is_empty());
        assert_eq!(d.files.len(), 1);
    }

    #[test]
    fn details_on_bad_hash_error() {
        let repo = TestRepo::with_initial_commit();
        assert!(commit_details(repo.path(), "deadbeef").is_err());
    }

    #[test]
    fn date_is_iso_8601() {
        let repo = TestRepo::with_initial_commit();
        let commits = log(repo.path(), 1, 0).unwrap();
        // git's %aI normalizes a +00:00 offset to the Z suffix.
        assert_eq!(commits[0].date, "2026-01-02T03:04:05Z");
    }
}
