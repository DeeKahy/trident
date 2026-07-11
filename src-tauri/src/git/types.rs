//! Serde types shared by every git command. These are the wire format
//! between Rust and the Svelte frontend, so field names serialize as
//! camelCase to match TypeScript conventions (mirrored in `src/lib/git.ts`).

use serde::Serialize;

/// Basic identity of an opened repository.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    /// Absolute path to the repository work-tree root.
    pub path: String,
    /// Directory name, used as the display title.
    pub name: String,
    /// Current branch name, or the short commit hash when detached.
    pub head: String,
    pub is_detached: bool,
}

/// The kind of change a file has, in the index or the work tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Typechange,
}

impl ChangeKind {
    /// Map a porcelain v2 status letter (M/A/D/R/C/T) to a kind.
    pub fn from_letter(letter: char) -> Option<Self> {
        match letter {
            'M' => Some(Self::Modified),
            'A' => Some(Self::Added),
            'D' => Some(Self::Deleted),
            'R' => Some(Self::Renamed),
            'C' => Some(Self::Copied),
            'T' => Some(Self::Typechange),
            _ => None,
        }
    }
}

/// One changed file in the status lists.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    /// Original path when the change is a rename or copy.
    pub orig_path: Option<String>,
    pub kind: ChangeKind,
    /// Line counts from --numstat; 0 for binary files.
    pub additions: u32,
    pub deletions: u32,
}

/// One tag, newest first. Annotated tags surface their own message
/// subject; lightweight tags fall back to the commit subject.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagInfo {
    pub name: String,
    /// Short hash of the commit the tag points at.
    pub hash: String,
    pub date: String,
    pub subject: String,
}

/// Branch/upstream summary from the `git status` headers.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchStatus {
    /// Branch name, or "(detached)" when HEAD is detached.
    pub head: String,
    /// Short hash of the current commit; empty on an unborn branch.
    pub oid: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}

/// Full result of `git status`, split into the lists the UI renders.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub branch: BranchStatus,
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub untracked: Vec<String>,
    pub conflicted: Vec<String>,
}

/// One commit in the history list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub email: String,
    /// Author date in strict ISO-8601, ready for `new Date(...)` in JS.
    pub date: String,
    pub parents: Vec<String>,
    pub subject: String,
    /// True when the commit has not reached the branch's upstream yet
    /// (or the branch has a remote but no upstream at all).
    pub local_only: bool,
}

/// Author or committer identity on a commit.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    pub name: String,
    pub email: String,
    /// Strict ISO-8601.
    pub date: String,
}

/// Everything the commit-details panel shows for one commit.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetails {
    pub hash: String,
    pub short_hash: String,
    pub author: Signature,
    pub committer: Signature,
    /// Full message: subject plus body.
    pub message: String,
    pub parents: Vec<String>,
    pub files: Vec<FileChange>,
}

/// Share of one language in a repository, for the project-card bar.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LangStat {
    pub name: String,
    /// Percent of tracked bytes, 0-100; the list sums to ~100.
    pub pct: u32,
}

/// Everything a project card on the hub shows about one repository.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSummary {
    pub name: String,
    pub path: String,
    pub branch: String,
    /// Files with any kind of pending change.
    pub changes: u32,
    pub ahead: u32,
    pub behind: u32,
    pub last_commit_date: Option<String>,
    /// URL of the origin remote, used to match local clones to their
    /// GitHub/GitLab counterparts.
    pub origin_url: Option<String>,
    pub langs: Vec<LangStat>,
}

/// A repository discovered by the folder scanner.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundRepo {
    pub path: String,
    pub name: String,
}

/// One local or remote branch.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub name: String,
    pub short_hash: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}
