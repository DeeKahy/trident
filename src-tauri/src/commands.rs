//! Tauri command wrappers around the `git` module. Kept as a thin layer:
//! deserialize arguments, call the pure function, serialize the result.
//! Commands are async so git subprocesses never block the main thread.

use std::path::PathBuf;

use crate::git::diff::DiffMode;
use crate::git::types::{BranchInfo, CommitInfo, RepoInfo, Status};
use crate::git::{branch, diff, log, repo, status, GitError};

type CmdResult<T> = Result<T, GitError>;

#[tauri::command]
pub async fn open_repo(path: String) -> CmdResult<RepoInfo> {
    repo::open_repo(&PathBuf::from(path))
}

#[tauri::command]
pub async fn git_status(repo_path: String) -> CmdResult<Status> {
    status::status(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_log(repo_path: String, limit: u32, skip: u32) -> CmdResult<Vec<CommitInfo>> {
    log::log(&PathBuf::from(repo_path), limit, skip)
}

#[tauri::command]
pub async fn git_branches(repo_path: String) -> CmdResult<Vec<BranchInfo>> {
    branch::branches(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_diff_file(repo_path: String, path: String, mode: DiffMode) -> CmdResult<String> {
    diff::diff_file(&PathBuf::from(repo_path), &path, mode)
}

#[tauri::command]
pub async fn git_commit_diff(repo_path: String, hash: String) -> CmdResult<String> {
    diff::commit_diff(&PathBuf::from(repo_path), &hash)
}
