//! Git backend: every git operation Trident performs lives in this module,
//! as plain functions that shell out to the system `git` and parse its
//! machine-readable output into typed structs. Tauri command wrappers live
//! in `crate::commands`; nothing in here depends on Tauri, so the whole
//! module is unit-testable against throwaway temp repos.

pub mod branch;
pub mod commit;
pub mod diff;
pub mod ignore;
pub mod log;
pub mod remote;
pub mod repo;
pub mod stage;
pub mod stash;
pub mod status;
pub mod summary;
pub mod tag;
pub mod types;

#[cfg(test)]
pub mod testutil;

use std::path::Path;
use std::process::Command;

use serde::Serialize;

/// Error returned by every git operation. Serializable so it crosses the
/// Tauri boundary intact and the frontend can show the real git message.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitError {
    pub message: String,
    pub command: String,
    pub exit_code: Option<i32>,
}

impl GitError {
    fn new(message: impl Into<String>, command: impl Into<String>, exit_code: Option<i32>) -> Self {
        Self {
            message: message.into(),
            command: command.into(),
            exit_code,
        }
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for GitError {}

pub type Result<T> = std::result::Result<T, GitError>;

/// Run git in `repo` and return stdout, treating any non-zero exit as an error.
pub fn run_git(repo: &Path, args: &[&str]) -> Result<String> {
    run_git_with_ok_codes(repo, args, &[0])
}

/// Run git in `repo`, accepting the listed exit codes as success.
/// `git diff` family exits 1 when differences exist, so diff callers pass `&[0, 1]`.
pub fn run_git_with_ok_codes(repo: &Path, args: &[&str], ok_codes: &[i32]) -> Result<String> {
    let display = format!("git {}", args.join(" "));

    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        // Read-only commands must not take optional locks (e.g. `git status`
        // refreshing the index), or the filesystem watcher would see our own
        // polls as repository changes and refresh forever.
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|e| GitError::new(format!("failed to spawn git: {e}"), display.clone(), None))?;

    let code = output.status.code();
    if !code.is_some_and(|c| ok_codes.contains(&c)) {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("git exited with status {code:?}")
        } else {
            stderr
        };
        return Err(GitError::new(message, display, code));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
