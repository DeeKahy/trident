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

/** Render an unknown thrown value (usually a GitError) as a message. */
export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as GitError).message);
  }
  return String(e);
}
