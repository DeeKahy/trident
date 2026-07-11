// Typed wrappers around the Rust git commands. The interfaces mirror the
// serde structs in src-tauri/src/git/types.rs (camelCase on the wire).

import { invoke } from "@tauri-apps/api/core";

export interface RepoInfo {
  path: string;
  name: string;
  head: string;
  isDetached: boolean;
}

export type ChangeKind =
  | "modified"
  | "added"
  | "deleted"
  | "renamed"
  | "copied"
  | "typechange";

export interface FileChange {
  path: string;
  origPath: string | null;
  kind: ChangeKind;
  additions: number;
  deletions: number;
}

export interface LangStat {
  name: string;
  pct: number;
}

export interface RepoSummary {
  name: string;
  path: string;
  branch: string;
  changes: number;
  ahead: number;
  behind: number;
  lastCommitDate: string | null;
  originUrl: string | null;
  langs: LangStat[];
}

export interface FoundRepo {
  path: string;
  name: string;
}

export interface GithubUser {
  login: string;
  name: string | null;
  avatarUrl: string;
  tokenSource: "env" | "keychain" | "gh";
}

export interface GithubRepo {
  name: string;
  fullName: string;
  private: boolean;
  fork: boolean;
  htmlUrl: string;
  cloneUrl: string;
  pushedAt: string | null;
  stars: number;
  language: string | null;
  description: string | null;
}

export interface TagInfo {
  name: string;
  hash: string;
  date: string;
  subject: string;
}

export interface BranchStatus {
  head: string;
  oid: string;
  upstream: string | null;
  ahead: number;
  behind: number;
}

export interface Status {
  branch: BranchStatus;
  staged: FileChange[];
  unstaged: FileChange[];
  untracked: string[];
  conflicted: string[];
}

export interface CommitInfo {
  hash: string;
  shortHash: string;
  author: string;
  email: string;
  date: string;
  parents: string[];
  subject: string;
  localOnly: boolean;
}

export interface Signature {
  name: string;
  email: string;
  date: string;
}

export interface CommitDetails {
  hash: string;
  shortHash: string;
  author: Signature;
  committer: Signature;
  message: string;
  parents: string[];
  files: FileChange[];
}

export interface BranchInfo {
  name: string;
  shortHash: string;
  isHead: boolean;
  isRemote: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
}

export interface GitError {
  message: string;
  command: string;
  exitCode: number | null;
}

export type DiffMode = "worktree" | "staged" | "untracked";

export function openRepo(path: string): Promise<RepoInfo> {
  return invoke("open_repo", { path });
}

export function gitStatus(repoPath: string): Promise<Status> {
  return invoke("git_status", { repoPath });
}

export function gitLog(repoPath: string, limit = 200, skip = 0, all = false): Promise<CommitInfo[]> {
  return invoke("git_log", { repoPath, limit, skip, all });
}

export function gitBranches(repoPath: string): Promise<BranchInfo[]> {
  return invoke("git_branches", { repoPath });
}

export function gitDiffFile(repoPath: string, path: string, mode: DiffMode): Promise<string> {
  return invoke("git_diff_file", { repoPath, path, mode });
}

export function gitCommitDiff(repoPath: string, hash: string): Promise<string> {
  return invoke("git_commit_diff", { repoPath, hash });
}

export function gitStageFile(repoPath: string, path: string): Promise<void> {
  return invoke("git_stage_file", { repoPath, path });
}

export function gitUnstageFile(repoPath: string, path: string): Promise<void> {
  return invoke("git_unstage_file", { repoPath, path });
}

export function gitStageAll(repoPath: string): Promise<void> {
  return invoke("git_stage_all", { repoPath });
}

export function gitUnstageAll(repoPath: string): Promise<void> {
  return invoke("git_unstage_all", { repoPath });
}

export function gitDiscardFile(
  repoPath: string,
  path: string,
  untracked: boolean
): Promise<void> {
  return invoke("git_discard_file", { repoPath, path, untracked });
}

export function gitCommit(repoPath: string, message: string, amend: boolean): Promise<string> {
  return invoke("git_commit", { repoPath, message, amend });
}

export function gitCreateBranch(
  repoPath: string,
  name: string,
  checkout: boolean
): Promise<void> {
  return invoke("git_create_branch", { repoPath, name, checkout });
}

export function gitSwitchBranch(repoPath: string, name: string): Promise<void> {
  return invoke("git_switch_branch", { repoPath, name });
}

export function gitDeleteBranch(repoPath: string, name: string, force: boolean): Promise<void> {
  return invoke("git_delete_branch", { repoPath, name, force });
}

export function gitFetch(repoPath: string): Promise<void> {
  return invoke("git_fetch", { repoPath });
}

export function gitPull(repoPath: string): Promise<void> {
  return invoke("git_pull", { repoPath });
}

export function gitPush(repoPath: string): Promise<void> {
  return invoke("git_push", { repoPath });
}

export function gitCommitDetails(repoPath: string, hash: string): Promise<CommitDetails> {
  return invoke("git_commit_details", { repoPath, hash });
}

/** Start (or move) the backend filesystem watcher to this repo. */
export function watchRepo(repoPath: string): Promise<void> {
  return invoke("watch_repo", { repoPath });
}

export function gitUntrackedLines(repoPath: string, path: string): Promise<number> {
  return invoke("git_untracked_lines", { repoPath, path });
}

export function gitTags(repoPath: string): Promise<TagInfo[]> {
  return invoke("git_tags", { repoPath });
}

export function gitCreateTag(
  repoPath: string,
  name: string,
  message: string,
  push: boolean
): Promise<void> {
  return invoke("git_create_tag", { repoPath, name, message, push });
}

export function gitStashList(repoPath: string): Promise<string[]> {
  return invoke("git_stash_list", { repoPath });
}

export function gitStashAll(repoPath: string, message: string): Promise<void> {
  return invoke("git_stash_all", { repoPath, message });
}

export function gitAddIgnore(repoPath: string, path: string): Promise<void> {
  return invoke("git_add_ignore", { repoPath, path });
}

export function gitRemoveIgnore(repoPath: string, path: string): Promise<void> {
  return invoke("git_remove_ignore", { repoPath, path });
}

export function gitUpdateMerge(repoPath: string): Promise<void> {
  return invoke("git_update_merge", { repoPath });
}

export function gitUpdateRebase(repoPath: string): Promise<void> {
  return invoke("git_update_rebase", { repoPath });
}

export function gitPublishBranch(repoPath: string, name: string): Promise<void> {
  return invoke("git_publish_branch", { repoPath, name });
}

export function gitRewordHead(repoPath: string, message: string): Promise<string> {
  return invoke("git_reword_head", { repoPath, message });
}

export function gitUndoLast(repoPath: string, keepChanges: boolean): Promise<void> {
  return invoke("git_undo_last", { repoPath, keepChanges });
}

export function gitRevert(repoPath: string, hash: string): Promise<void> {
  return invoke("git_revert", { repoPath, hash });
}

export function gitSwitchDetached(repoPath: string, hash: string): Promise<void> {
  return invoke("git_switch_detached", { repoPath, hash });
}

export function repoSummary(path: string): Promise<RepoSummary> {
  return invoke("repo_summary", { path });
}

export function cloneRepo(url: string, dest: string): Promise<RepoInfo> {
  return invoke("clone_repo", { url, dest });
}

export function initRepo(path: string): Promise<RepoInfo> {
  return invoke("init_repo", { path });
}

export function openInEditor(path: string): Promise<void> {
  return invoke("open_in_editor", { path });
}

export function openInTerminal(path: string): Promise<void> {
  return invoke("open_in_terminal", { path });
}

export interface LangDetail {
  name: string;
  files: number;
  code: number;
  comments: number;
  blanks: number;
}

export interface Contributor {
  name: string;
  email: string;
  commits: number;
}

export interface CodeStats {
  files: number;
  code: number;
  comments: number;
  blanks: number;
  commits: number;
  firstCommitDate: string | null;
  languages: LangDetail[];
  contributors: Contributor[];
}

export interface ScanReport {
  repos: FoundRepo[];
  unreadableRoots: string[];
}

export function scanRepos(): Promise<ScanReport> {
  return invoke("scan_repos");
}

export function scanFolder(path: string): Promise<FoundRepo[]> {
  return invoke("scan_folder", { path });
}

export function codeStats(repoPath: string): Promise<CodeStats> {
  return invoke("code_stats", { repoPath });
}

export function githubAccount(): Promise<GithubUser | null> {
  return invoke("github_account");
}

export function githubRepos(): Promise<GithubRepo[]> {
  return invoke("github_repos");
}

export function githubConnect(token: string): Promise<GithubUser> {
  return invoke("github_connect", { token });
}

export function githubDisconnect(): Promise<void> {
  return invoke("github_disconnect");
}

/** Render an unknown thrown value (usually a GitError) as a message. */
export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as GitError).message);
  }
  return String(e);
}
