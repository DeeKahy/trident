<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import DiffView from "$lib/DiffView.svelte";
  import {
    openRepo,
    gitStatus,
    gitLog,
    gitBranches,
    gitDiffFile,
    gitCommitDiff,
    errorMessage,
    type RepoInfo,
    type Status,
    type CommitInfo,
    type BranchInfo,
    type DiffMode,
    type FileChange,
  } from "$lib/git";

  let repo = $state<RepoInfo | null>(null);
  let status = $state<Status | null>(null);
  let commits = $state<CommitInfo[]>([]);
  let branches = $state<BranchInfo[]>([]);
  let view = $state<"changes" | "history">("changes");
  let error = $state<string | null>(null);
  let manualPath = $state("");

  let diffText = $state("");
  let diffTitle = $state("");
  let selectedKey = $state<string | null>(null);

  let localBranches = $derived(branches.filter((b) => !b.isRemote));
  let remoteBranches = $derived(branches.filter((b) => b.isRemote));
  let changeCount = $derived(
    status
      ? status.staged.length +
        status.unstaged.length +
        status.untracked.length +
        status.conflicted.length
      : 0
  );

  async function openFromDialog() {
    const picked = await openDialog({ directory: true, title: "Open repository" });
    if (typeof picked === "string") await openPath(picked);
  }

  async function openPath(path: string) {
    error = null;
    try {
      repo = await openRepo(path);
      selectedKey = null;
      diffText = "";
      diffTitle = "";
      await refresh();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function refresh() {
    if (!repo) return;
    error = null;
    try {
      [status, commits, branches] = await Promise.all([
        gitStatus(repo.path),
        gitLog(repo.path),
        gitBranches(repo.path),
      ]);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function selectFile(path: string, mode: DiffMode) {
    if (!repo) return;
    selectedKey = `${mode}:${path}`;
    diffTitle = path;
    try {
      diffText = await gitDiffFile(repo.path, path, mode);
    } catch (e) {
      diffText = "";
      error = errorMessage(e);
    }
  }

  async function selectCommit(commit: CommitInfo) {
    if (!repo) return;
    selectedKey = commit.hash;
    diffTitle = `${commit.shortHash} ${commit.subject}`;
    try {
      diffText = await gitCommitDiff(repo.path, commit.hash);
    } catch (e) {
      diffText = "";
      error = errorMessage(e);
    }
  }

  function kindBadge(change: FileChange): string {
    return { modified: "M", added: "A", deleted: "D", renamed: "R", copied: "C", typechange: "T" }[
      change.kind
    ];
  }

  function commitDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }
</script>

{#snippet fileList(title: string, files: FileChange[], mode: DiffMode)}
  {#if files.length > 0}
    <div class="group-title">{title} <span class="count">{files.length}</span></div>
    {#each files as file (mode + file.path)}
      <button
        class="row"
        class:selected={selectedKey === `${mode}:${file.path}`}
        onclick={() => selectFile(file.path, mode)}
      >
        <span class="badge {file.kind}">{kindBadge(file)}</span>
        <span class="row-label" title={file.path}>
          {#if file.origPath}{file.origPath} → {/if}{file.path}
        </span>
      </button>
    {/each}
  {/if}
{/snippet}

{#if repo === null}
  <main class="welcome">
    <h1>Trident</h1>
    <p>A fast, friendly git client. Open a repository to get started.</p>
    <button class="primary" onclick={openFromDialog}>Open repository…</button>
    <form
      class="manual"
      onsubmit={(e) => {
        e.preventDefault();
        if (manualPath.trim()) openPath(manualPath.trim());
      }}
    >
      <input placeholder="…or paste a repo path" bind:value={manualPath} />
      <button type="submit">Open</button>
    </form>
    {#if error}<div class="error">{error}</div>{/if}
  </main>
{:else}
  <main class="app">
    <header class="toolbar">
      <button onclick={openFromDialog} title="Open another repository">Open…</button>
      <span class="repo-name">{repo.name}</span>
      {#if status}
        <span class="branch-chip" title="Current branch">
          {status.branch.head}
          {#if status.branch.ahead > 0}<span class="ahead">↑{status.branch.ahead}</span>{/if}
          {#if status.branch.behind > 0}<span class="behind">↓{status.branch.behind}</span>{/if}
        </span>
        {#if status.branch.upstream}
          <span class="upstream" title="Upstream">⇄ {status.branch.upstream}</span>
        {/if}
      {/if}
      <span class="spacer"></span>
      <button onclick={refresh} title="Refresh">⟳ Refresh</button>
    </header>

    {#if error}<div class="error banner">{error}</div>{/if}

    <div class="columns">
      <nav class="sidebar">
        <div class="group-title">Branches</div>
        {#each localBranches as branch (branch.name)}
          <div class="row static" class:head={branch.isHead}>
            <span class="row-label" title={branch.name}>
              {branch.isHead ? "● " : "   "}{branch.name}
            </span>
            {#if branch.ahead > 0 || branch.behind > 0}
              <span class="track">
                {#if branch.ahead > 0}↑{branch.ahead}{/if}
                {#if branch.behind > 0}↓{branch.behind}{/if}
              </span>
            {/if}
          </div>
        {/each}
        {#if remoteBranches.length > 0}
          <div class="group-title">Remotes</div>
          {#each remoteBranches as branch (branch.name)}
            <div class="row static remote">
              <span class="row-label" title={branch.name}>{branch.name}</span>
            </div>
          {/each}
        {/if}
      </nav>

      <section class="list-pane">
        <div class="tabs">
          <button class:active={view === "changes"} onclick={() => (view = "changes")}>
            Changes {#if changeCount > 0}<span class="count">{changeCount}</span>{/if}
          </button>
          <button class:active={view === "history"} onclick={() => (view = "history")}>
            History
          </button>
        </div>

        {#if view === "changes"}
          <div class="scroll">
            {#if status}
              {#if status.conflicted.length > 0}
                <div class="group-title conflict">
                  Conflicts <span class="count">{status.conflicted.length}</span>
                </div>
                {#each status.conflicted as path (path)}
                  <button
                    class="row"
                    class:selected={selectedKey === `worktree:${path}`}
                    onclick={() => selectFile(path, "worktree")}
                  >
                    <span class="badge conflictbadge">!</span>
                    <span class="row-label">{path}</span>
                  </button>
                {/each}
              {/if}
              {@render fileList("Staged", status.staged, "staged")}
              {@render fileList("Unstaged", status.unstaged, "worktree")}
              {#if status.untracked.length > 0}
                <div class="group-title">
                  Untracked <span class="count">{status.untracked.length}</span>
                </div>
                {#each status.untracked as path (path)}
                  <button
                    class="row"
                    class:selected={selectedKey === `untracked:${path}`}
                    onclick={() => selectFile(path, "untracked")}
                  >
                    <span class="badge added">?</span>
                    <span class="row-label" title={path}>{path}</span>
                  </button>
                {/each}
              {/if}
              {#if changeCount === 0}
                <div class="empty-note">Working tree clean</div>
              {/if}
            {/if}
          </div>
        {:else}
          <div class="scroll">
            {#each commits as commit (commit.hash)}
              <button
                class="row commit"
                class:selected={selectedKey === commit.hash}
                onclick={() => selectCommit(commit)}
              >
                <span class="commit-subject" title={commit.subject}>{commit.subject}</span>
                <span class="commit-meta">
                  <code>{commit.shortHash}</code>
                  {commit.author} · {commitDate(commit.date)}
                  {#if commit.parents.length > 1}· merge{/if}
                </span>
              </button>
            {:else}
              <div class="empty-note">No commits yet</div>
            {/each}
          </div>
        {/if}
      </section>

      <section class="diff-pane">
        {#if diffTitle}
          <div class="diff-title" title={diffTitle}>{diffTitle}</div>
        {/if}
        <div class="diff-body">
          <DiffView
            diff={diffText}
            emptyMessage={selectedKey ? "No changes to show" : "Select a file or commit"}
          />
        </div>
      </section>
    </div>
  </main>
{/if}

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    background: var(--bg, #0d1117);
    color: var(--fg, #e6edf3);
    font-family:
      -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }

  .welcome {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
  }
  .welcome h1 {
    font-size: 2.5rem;
    margin: 0;
    letter-spacing: -0.02em;
  }
  .welcome p {
    color: var(--fg-muted, #8b949e);
    margin: 0 0 1rem;
  }
  .manual {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  .manual input {
    width: 320px;
  }

  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border, #30363d);
    flex: none;
  }
  .repo-name {
    font-weight: 600;
  }
  .branch-chip {
    background: var(--chip, #21262d);
    border: 1px solid var(--border, #30363d);
    border-radius: 999px;
    padding: 0.1rem 0.6rem;
    font-size: 12px;
  }
  .ahead {
    color: #3fb950;
    margin-left: 0.3rem;
  }
  .behind {
    color: #f85149;
    margin-left: 0.3rem;
  }
  .upstream {
    color: var(--fg-muted, #8b949e);
    font-size: 12px;
  }
  .spacer {
    flex: 1;
  }

  .columns {
    flex: 1;
    display: grid;
    grid-template-columns: 200px 320px 1fr;
    min-height: 0;
  }
  .sidebar {
    border-right: 1px solid var(--border, #30363d);
    overflow-y: auto;
    padding: 0.5rem 0;
  }
  .list-pane {
    border-right: 1px solid var(--border, #30363d);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border, #30363d);
    flex: none;
  }
  .tabs button {
    flex: 1;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--fg-muted, #8b949e);
    padding: 0.5rem;
    font-size: 13px;
    cursor: pointer;
  }
  .tabs button.active {
    color: var(--fg, #e6edf3);
    border-bottom-color: var(--accent, #58a6ff);
  }
  .scroll {
    overflow-y: auto;
    flex: 1;
    padding: 0.25rem 0;
  }

  .group-title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-muted, #8b949e);
    padding: 0.6rem 0.75rem 0.25rem;
  }
  .group-title.conflict {
    color: #f85149;
  }
  .count {
    background: var(--chip, #21262d);
    border-radius: 999px;
    padding: 0 0.4rem;
    font-size: 11px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    color: var(--fg, #e6edf3);
    padding: 0.3rem 0.75rem;
    font-size: 13px;
    cursor: pointer;
    box-sizing: border-box;
  }
  .row:hover:not(.static) {
    background: var(--hover, #161b22);
  }
  .row.selected {
    background: var(--selected, #1f2937);
  }
  .row.static {
    cursor: default;
  }
  .row.head {
    font-weight: 600;
  }
  .row.remote {
    color: var(--fg-muted, #8b949e);
  }
  .row-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .track {
    font-size: 11px;
    color: var(--fg-muted, #8b949e);
  }

  .badge {
    font-family: ui-monospace, monospace;
    font-size: 11px;
    font-weight: 700;
    width: 1.2rem;
    text-align: center;
    flex: none;
    border-radius: 3px;
  }
  .badge.modified {
    color: #d29922;
  }
  .badge.added {
    color: #3fb950;
  }
  .badge.deleted {
    color: #f85149;
  }
  .badge.renamed,
  .badge.copied,
  .badge.typechange {
    color: #58a6ff;
  }
  .badge.conflictbadge {
    color: #f85149;
  }

  .row.commit {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    padding: 0.4rem 0.75rem;
  }
  .commit-subject {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }
  .commit-meta {
    font-size: 11px;
    color: var(--fg-muted, #8b949e);
  }
  .commit-meta code {
    color: var(--accent, #58a6ff);
    font-size: 11px;
  }

  .diff-pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .diff-title {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border, #30363d);
    font-size: 12px;
    font-family: ui-monospace, monospace;
    color: var(--fg-muted, #8b949e);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
  }
  .diff-body {
    flex: 1;
    min-height: 0;
  }

  .empty-note {
    color: var(--fg-muted, #8b949e);
    font-size: 13px;
    padding: 1rem 0.75rem;
  }

  .error {
    color: #f85149;
    font-size: 13px;
    max-width: 480px;
  }
  .error.banner {
    padding: 0.4rem 0.75rem;
    background: rgba(248, 81, 73, 0.1);
    border-bottom: 1px solid var(--border, #30363d);
    max-width: none;
  }

  button,
  input {
    font: inherit;
  }
  button.primary,
  .manual button,
  .toolbar button {
    background: var(--chip, #21262d);
    color: var(--fg, #e6edf3);
    border: 1px solid var(--border, #30363d);
    border-radius: 6px;
    padding: 0.35rem 0.8rem;
    cursor: pointer;
    font-size: 13px;
  }
  button.primary {
    background: var(--accent-bg, #238636);
    border-color: transparent;
    font-size: 14px;
  }
  button.primary:hover {
    background: #2ea043;
  }
  .manual input {
    background: var(--bg, #0d1117);
    border: 1px solid var(--border, #30363d);
    border-radius: 6px;
    color: var(--fg, #e6edf3);
    padding: 0.35rem 0.6rem;
    font-size: 13px;
  }
</style>
