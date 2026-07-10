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

export function gitLog(repoPath: string, limit = 200, skip = 0): Promise<CommitInfo[]> {
  return invoke("git_log", { repoPath, limit, skip });
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

/** Render an unknown thrown value (usually a GitError) as a message. */
export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as GitError).message);
  }
  return String(e);
}
