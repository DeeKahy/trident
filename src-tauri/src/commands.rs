//! Tauri command wrappers around the `git` module. Kept as a thin layer:
//! deserialize arguments, call the pure function, serialize the result.
//! Commands are async so git subprocesses never block the main thread.

use std::path::PathBuf;

use crate::git::diff::DiffMode;
use crate::git::types::{BranchInfo, CommitInfo, RepoInfo, Status};
use crate::git::{branch, commit, diff, log, remote, repo, stage, status, GitError};

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

#[tauri::command]
pub async fn git_stage_file(repo_path: String, path: String) -> CmdResult<()> {
    stage::stage_file(&PathBuf::from(repo_path), &path)
}

#[tauri::command]
pub async fn git_unstage_file(repo_path: String, path: String) -> CmdResult<()> {
    stage::unstage_file(&PathBuf::from(repo_path), &path)
}

#[tauri::command]
pub async fn git_stage_all(repo_path: String) -> CmdResult<()> {
    stage::stage_all(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_unstage_all(repo_path: String) -> CmdResult<()> {
    stage::unstage_all(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_discard_file(repo_path: String, path: String, untracked: bool) -> CmdResult<()> {
    stage::discard_file(&PathBuf::from(repo_path), &path, untracked)
}

#[tauri::command]
pub async fn git_commit(repo_path: String, message: String, amend: bool) -> CmdResult<String> {
    commit::commit(&PathBuf::from(repo_path), &message, amend)
}

#[tauri::command]
pub async fn git_create_branch(repo_path: String, name: String, checkout: bool) -> CmdResult<()> {
    branch::create_branch(&PathBuf::from(repo_path), &name, checkout)
}

#[tauri::command]
pub async fn git_switch_branch(repo_path: String, name: String) -> CmdResult<()> {
    branch::switch_branch(&PathBuf::from(repo_path), &name)
}

#[tauri::command]
pub async fn git_delete_branch(repo_path: String, name: String, force: bool) -> CmdResult<()> {
    branch::delete_branch(&PathBuf::from(repo_path), &name, force)
}

#[tauri::command]
pub async fn git_fetch(repo_path: String) -> CmdResult<()> {
    remote::fetch(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_pull(repo_path: String) -> CmdResult<()> {
    remote::pull(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_push(repo_path: String) -> CmdResult<()> {
    remote::push(&PathBuf::from(repo_path))
}
