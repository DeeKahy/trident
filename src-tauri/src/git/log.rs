//! Commit history via `git log` with a custom field-separated format.
//!
//! Fields are joined with the ASCII unit separator (0x1f) and records with
//! the record separator (0x1e), neither of which can appear in hashes,
//! names, or a single-line subject.

use std::path::Path;

use super::types::CommitInfo;
use super::{run_git, Result};

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
    fn date_is_iso_8601() {
        let repo = TestRepo::with_initial_commit();
        let commits = log(repo.path(), 1, 0).unwrap();
        // git's %aI normalizes a +00:00 offset to the Z suffix.
        assert_eq!(commits[0].date, "2026-01-02T03:04:05Z");
    }
}
