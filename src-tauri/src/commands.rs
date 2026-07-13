//! Tauri command wrappers around the `git` module. Kept as a thin layer:
//! deserialize arguments, call the pure function, serialize the result.
//! Commands are async so git subprocesses never block the main thread.

use std::path::PathBuf;

use crate::git::diff::DiffMode;
use crate::git::types::{
    BranchInfo, CommitDetails, CommitInfo, FoundRepo, RepoInfo, RepoSummary, ScanReport, Status,
    TagInfo,
};
use crate::git::{
    branch, commit, diff, ignore, log, remote, repo, scan, stage, stash, status, summary, tag,
    GitError,
};

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
pub async fn repo_summary(path: String) -> CmdResult<RepoSummary> {
    summary::repo_summary(&PathBuf::from(path))
}

#[tauri::command]
pub async fn clone_repo(url: String, dest: String) -> CmdResult<RepoInfo> {
    repo::clone_repo(&url, &PathBuf::from(dest))
}

#[tauri::command]
pub async fn init_repo(path: String) -> CmdResult<RepoInfo> {
    repo::init_repo(&PathBuf::from(path))
}

#[tauri::command]
pub async fn scan_repos() -> CmdResult<ScanReport> {
    Ok(scan::scan_with_report(&scan::default_roots()))
}

#[tauri::command]
pub async fn scan_folder(path: String) -> CmdResult<Vec<FoundRepo>> {
    Ok(scan::scan_for_repos(&[PathBuf::from(path)]))
}

#[tauri::command]
pub async fn code_stats(repo_path: String) -> CmdResult<crate::git::stats::CodeStats> {
    crate::git::stats::code_stats(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_log(
    repo_path: String,
    limit: u32,
    skip: u32,
    all: bool,
) -> CmdResult<Vec<CommitInfo>> {
    log::log(&PathBuf::from(repo_path), limit, skip, all)
}

#[tauri::command]
pub async fn git_branches(repo_path: String) -> CmdResult<Vec<BranchInfo>> {
    branch::branches(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_diff_file(
    repo_path: String,
    path: String,
    mode: DiffMode,
    full: bool,
) -> CmdResult<String> {
    diff::diff_file(&PathBuf::from(repo_path), &path, mode, full)
}

#[tauri::command]
pub async fn git_commit_diff(repo_path: String, hash: String, full: bool) -> CmdResult<String> {
    diff::commit_diff(&PathBuf::from(repo_path), &hash, full)
}

#[tauri::command]
pub async fn git_commit_details(repo_path: String, hash: String) -> CmdResult<CommitDetails> {
    log::commit_details(&PathBuf::from(repo_path), &hash)
}

#[tauri::command]
pub async fn git_untracked_lines(repo_path: String, path: String) -> CmdResult<u32> {
    Ok(status::untracked_line_count(&PathBuf::from(repo_path), &path))
}

#[tauri::command]
pub async fn git_tags(repo_path: String) -> CmdResult<Vec<TagInfo>> {
    tag::list_tags(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_create_tag(
    repo_path: String,
    name: String,
    message: String,
    push: bool,
) -> CmdResult<()> {
    tag::create_tag(&PathBuf::from(repo_path), &name, &message, push)
}

#[tauri::command]
pub async fn git_stash_list(repo_path: String) -> CmdResult<Vec<String>> {
    stash::stash_list(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_stash_all(repo_path: String, message: String) -> CmdResult<()> {
    stash::stash_all(&PathBuf::from(repo_path), &message)
}

#[tauri::command]
pub async fn git_add_ignore(repo_path: String, path: String) -> CmdResult<()> {
    ignore::add_to_gitignore(&PathBuf::from(repo_path), &path)
}

#[tauri::command]
pub async fn git_remove_ignore(repo_path: String, path: String) -> CmdResult<()> {
    ignore::remove_from_gitignore(&PathBuf::from(repo_path), &path)
}

#[tauri::command]
pub async fn git_update_merge(repo_path: String) -> CmdResult<()> {
    remote::update_with_merge(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_update_rebase(repo_path: String) -> CmdResult<()> {
    remote::update_with_rebase(&PathBuf::from(repo_path))
}

#[tauri::command]
pub async fn git_publish_branch(repo_path: String, name: String) -> CmdResult<()> {
    remote::publish_branch(&PathBuf::from(repo_path), &name)
}

#[tauri::command]
pub async fn git_reword_head(repo_path: String, message: String) -> CmdResult<String> {
    commit::reword_head(&PathBuf::from(repo_path), &message)
}

#[tauri::command]
pub async fn git_undo_last(repo_path: String, keep_changes: bool) -> CmdResult<()> {
    commit::undo_last_commit(&PathBuf::from(repo_path), keep_changes)
}

#[tauri::command]
pub async fn git_revert(repo_path: String, hash: String) -> CmdResult<()> {
    commit::revert_commit(&PathBuf::from(repo_path), &hash)
}

#[tauri::command]
pub async fn git_switch_detached(repo_path: String, hash: String) -> CmdResult<()> {
    branch::switch_detached(&PathBuf::from(repo_path), &hash)
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
