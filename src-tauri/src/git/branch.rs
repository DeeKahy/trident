//! Branch listing via `git for-each-ref`.

use std::path::Path;

use super::types::BranchInfo;
use super::{run_git, Result};

/// List local and remote branches. The current branch is flagged `is_head`;
/// remote HEAD pointers (e.g. `origin/HEAD`) are filtered out.
pub fn branches(repo: &Path) -> Result<Vec<BranchInfo>> {
    // %1f is the ASCII unit separator; refnames cannot contain control
    // characters or newlines, so line/field splitting is unambiguous.
    let format = "%(HEAD)%1f%(refname)%1f%(refname:short)%1f%(objectname:short)%1f%(upstream:short)%1f%(upstream:track)";
    let format_arg = format!("--format={format}");
    let raw = run_git(
        repo,
        &["for-each-ref", &format_arg, "refs/heads", "refs/remotes"],
    )?;

    Ok(raw.lines().filter_map(parse_line).collect())
}

fn parse_line(line: &str) -> Option<BranchInfo> {
    let mut fields = line.split('\u{1f}');
    let head_marker = fields.next()?;
    let refname = fields.next()?;
    let name = fields.next()?.to_string();
    let short_hash = fields.next()?.to_string();
    let upstream = fields.next()?.to_string();
    let track = fields.next()?.to_string();

    let is_remote = refname.starts_with("refs/remotes/");
    if is_remote && refname.ends_with("/HEAD") {
        return None;
    }

    let (ahead, behind) = parse_track(&track);
    Some(BranchInfo {
        name,
        short_hash,
        is_head: head_marker == "*",
        is_remote,
        upstream: (!upstream.is_empty()).then_some(upstream),
        ahead,
        behind,
    })
}

/// Parse `%(upstream:track)` output like `[ahead 1, behind 2]` or `[gone]`.
fn parse_track(track: &str) -> (u32, u32) {
    let inner = track.trim_start_matches('[').trim_end_matches(']');
    let mut ahead = 0;
    let mut behind = 0;
    for part in inner.split(", ") {
        if let Some(n) = part.strip_prefix("ahead ") {
            ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            behind = n.parse().unwrap_or(0);
        }
    }
    (ahead, behind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;

    #[test]
    fn lists_local_branches_and_flags_head() {
        let repo = TestRepo::with_initial_commit();
        repo.git(&["branch", "feature/one"]);

        let all = branches(repo.path()).unwrap();
        assert_eq!(all.len(), 2);

        let main = all.iter().find(|b| b.name == "main").unwrap();
        assert!(main.is_head);
        assert!(!main.is_remote);
        assert!(!main.short_hash.is_empty());

        let feature = all.iter().find(|b| b.name == "feature/one").unwrap();
        assert!(!feature.is_head);
    }

    #[test]
    fn reports_upstream_and_ahead_behind() {
        let repo = TestRepo::with_initial_commit();
        repo.git(&["branch", "base"]);
        repo.write("more.txt", "x\n");
        repo.git(&["add", "more.txt"]);
        repo.commit("second commit");
        repo.git(&["branch", "--set-upstream-to=base"]);

        let all = branches(repo.path()).unwrap();
        let main = all.iter().find(|b| b.name == "main").unwrap();
        assert_eq!(main.upstream.as_deref(), Some("base"));
        assert_eq!(main.ahead, 1);
        assert_eq!(main.behind, 0);
    }

    #[test]
    fn empty_repo_has_no_branches() {
        let repo = TestRepo::empty();
        let all = branches(repo.path()).unwrap();
        assert!(all.is_empty());
    }
}
