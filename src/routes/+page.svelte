<script lang="ts">
  import { open as openDialog, confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import DiffView from "$lib/DiffView.svelte";
  import Avatar from "$lib/Avatar.svelte";
  import {
    gitCommitDetails,
    watchRepo,
    type CommitDetails,
    openRepo,
    gitStatus,
    gitLog,
    gitBranches,
    gitDiffFile,
    gitCommitDiff,
    gitStageFile,
    gitUnstageFile,
    gitStageAll,
    gitUnstageAll,
    gitDiscardFile,
    gitCommit,
    gitCreateBranch,
    gitSwitchBranch,
    gitDeleteBranch,
    gitFetch,
    gitPull,
    gitPush,
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

  let commitSubject = $state("");
  let commitBody = $state("");
  let amend = $state(false);
  let newBranchName = $state("");
  let remoteBusy = $state<"fetch" | "pull" | "push" | null>(null);
  let details = $state<CommitDetails | null>(null);
  let logExhausted = $state(false);
  let loadingMore = $state(false);

  const LOG_PAGE = 200;
  const RECENT_KEY = "trident.recentRepos";

  interface RecentRepo {
    path: string;
    name: string;
  }

  let recentRepos = $state<RecentRepo[]>(loadRecent());

  function loadRecent(): RecentRepo[] {
    try {
      return JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    } catch {
      return [];
    }
  }

  function rememberRepo(info: RepoInfo) {
    recentRepos = [
      { path: info.path, name: info.name },
      ...recentRepos.filter((r) => r.path !== info.path),
    ].slice(0, 8);
    localStorage.setItem(RECENT_KEY, JSON.stringify(recentRepos));
  }

  // Auto-refresh: the backend watcher emits repo-changed on any relevant
  // disk change; debounce bursts (checkouts touch hundreds of files).
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const unlisten = listen("repo-changed", () => {
      clearTimeout(refreshTimer);
      refreshTimer = setTimeout(refresh, 400);
    });
    return () => {
      clearTimeout(refreshTimer);
      unlisten.then((u) => u());
    };
  });

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
  let canCommit = $derived(
    commitSubject.trim().length > 0 && status !== null && (status.staged.length > 0 || amend)
  );

  async function openFromDialog() {
    const picked = await openDialog({ directory: true, title: "Open repository" });
    if (typeof picked === "string") await openPath(picked);
  }

  async function openPath(path: string) {
    error = null;
    try {
      repo = await openRepo(path);
      rememberRepo(repo);
      clearDiff();
      logExhausted = false;
      commits = [];
      await refresh();
      await watchRepo(repo.path);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function clearDiff() {
    selectedKey = null;
    diffText = "";
    diffTitle = "";
    details = null;
  }

  async function refresh() {
    if (!repo) return;
    error = null;
    try {
      // Reload at least as much history as is already on screen so
      // "load more" progress survives an auto-refresh.
      const limit = Math.max(commits.length, LOG_PAGE);
      [status, commits, branches] = await Promise.all([
        gitStatus(repo.path),
        gitLog(repo.path, limit),
        gitBranches(repo.path),
      ]);
      await resyncDiff();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** After a refresh, bring the open diff up to date with the new state. */
  async function resyncDiff() {
    if (!repo || !selectedKey) return;
    const colon = selectedKey.indexOf(":");
    if (colon === -1) return; // a commit diff; commits are immutable
    const mode = selectedKey.slice(0, colon) as DiffMode;
    const path = selectedKey.slice(colon + 1);
    try {
      diffText = await gitDiffFile(repo.path, path, mode);
    } catch {
      // The file left this state (staged, committed, discarded): close it.
      clearDiff();
    }
  }

  async function loadMore() {
    if (!repo || loadingMore) return;
    loadingMore = true;
    try {
      const older = await gitLog(repo.path, LOG_PAGE, commits.length);
      commits = [...commits, ...older];
      if (older.length < LOG_PAGE) logExhausted = true;
    } catch (e) {
      error = errorMessage(e);
    }
    loadingMore = false;
  }

  /** Run a mutating action, surface its error, refresh the panes. */
  async function act(fn: () => Promise<unknown>) {
    if (!repo) return;
    error = null;
    try {
      await fn();
    } catch (e) {
      error = errorMessage(e);
    }
    await refresh();
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
    details = null;
    try {
      [diffText, details] = await Promise.all([
        gitCommitDiff(repo.path, commit.hash),
        gitCommitDetails(repo.path, commit.hash),
      ]);
    } catch (e) {
      diffText = "";
      error = errorMessage(e);
    }
  }

  async function stageFile(path: string) {
    await act(() => gitStageFile(repo!.path, path));
    clearDiff();
  }

  async function unstageFile(path: string) {
    await act(() => gitUnstageFile(repo!.path, path));
    clearDiff();
  }

  async function discardFile(path: string, untracked: boolean) {
    const verb = untracked ? "Delete untracked file" : "Discard changes to";
    const ok = await confirmDialog(`${verb} "${path}"? This cannot be undone.`, {
      title: "Trident",
      kind: "warning",
    });
    if (!ok) return;
    await act(() => gitDiscardFile(repo!.path, path, untracked));
    clearDiff();
  }

  async function doCommit() {
    if (!canCommit) return;
    const body = commitBody.trim();
    const message = body ? `${commitSubject.trim()}\n\n${body}` : commitSubject.trim();
    await act(() => gitCommit(repo!.path, message, amend));
    if (!error) {
      commitSubject = "";
      commitBody = "";
      amend = false;
      clearDiff();
    }
  }

  async function switchTo(branch: BranchInfo) {
    if (branch.isHead || branch.isRemote) return;
    await act(() => gitSwitchBranch(repo!.path, branch.name));
    clearDiff();
  }

  async function createBranch() {
    const name = newBranchName.trim();
    if (!name) return;
    await act(() => gitCreateBranch(repo!.path, name, true));
    if (!error) newBranchName = "";
  }

  async function deleteBranch(branch: BranchInfo) {
    const ok = await confirmDialog(`Delete branch "${branch.name}"?`, {
      title: "Trident",
      kind: "warning",
    });
    if (!ok || !repo) return;
    try {
      await gitDeleteBranch(repo.path, branch.name, false);
    } catch (e) {
      const message = errorMessage(e);
      if (message.includes("not fully merged")) {
        const force = await confirmDialog(
          `"${branch.name}" has commits that are not merged anywhere else. Delete anyway and lose them?`,
          { title: "Trident", kind: "warning" }
        );
        if (force) {
          await act(() => gitDeleteBranch(repo!.path, branch.name, true));
          return;
        }
      } else {
        error = message;
      }
    }
    await refresh();
  }

  async function remoteOp(kind: "fetch" | "pull" | "push") {
    if (!repo || remoteBusy) return;
    remoteBusy = kind;
    const ops = { fetch: gitFetch, pull: gitPull, push: gitPush };
    await act(() => ops[kind](repo!.path));
    remoteBusy = null;
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

  function fullDate(iso: string): string {
    return new Date(iso).toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }
</script>

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
    {#if recentRepos.length > 0}
      <div class="recent">
        <div class="group-title">Recent</div>
        {#each recentRepos as recent (recent.path)}
          <button class="recent-row" onclick={() => openPath(recent.path)}>
            <span class="recent-name">{recent.name}</span>
            <span class="recent-path" title={recent.path}>{recent.path}</span>
          </button>
        {/each}
      </div>
    {/if}
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
      <button onclick={() => remoteOp("fetch")} disabled={remoteBusy !== null}>
        {remoteBusy === "fetch" ? "Fetching…" : "Fetch"}
      </button>
      <button onclick={() => remoteOp("pull")} disabled={remoteBusy !== null}>
        {remoteBusy === "pull" ? "Pulling…" : "Pull"}
      </button>
      <button onclick={() => remoteOp("push")} disabled={remoteBusy !== null}>
        {remoteBusy === "push"
          ? "Pushing…"
          : status && status.branch.ahead > 0
            ? `Push (${status.branch.ahead})`
            : "Push"}
      </button>
      <button onclick={refresh} title="Refresh">⟳</button>
    </header>

    {#if error}<div class="error banner">{error}</div>{/if}

    <div class="columns">
      <nav class="sidebar">
        <div class="group-title">Branches</div>
        {#each localBranches as branch (branch.name)}
          <div class="row" class:head={branch.isHead}>
            <button
              class="row-main"
              disabled={branch.isHead}
              title={branch.isHead ? branch.name : `Switch to ${branch.name}`}
              onclick={() => switchTo(branch)}
            >
              <span class="row-label">{branch.isHead ? "● " : ""}{branch.name}</span>
              {#if branch.ahead > 0 || branch.behind > 0}
                <span class="track">
                  {#if branch.ahead > 0}↑{branch.ahead}{/if}
                  {#if branch.behind > 0}↓{branch.behind}{/if}
                </span>
              {/if}
            </button>
            {#if !branch.isHead}
              <span class="row-actions">
                <button class="mini danger" title="Delete branch" onclick={() => deleteBranch(branch)}>
                  ×
                </button>
              </span>
            {/if}
          </div>
        {/each}
        <form
          class="new-branch"
          onsubmit={(e) => {
            e.preventDefault();
            createBranch();
          }}
        >
          <input placeholder="new branch…" bind:value={newBranchName} />
          <button type="submit" class="mini" disabled={!newBranchName.trim()}>+</button>
        </form>
        {#if remoteBranches.length > 0}
          <div class="group-title">Remotes</div>
          {#each remoteBranches as branch (branch.name)}
            <div class="row remote">
              <span class="row-label static-label" title={branch.name}>{branch.name}</span>
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
                  <div class="row" class:selected={selectedKey === `worktree:${path}`}>
                    <button class="row-main" onclick={() => selectFile(path, "worktree")}>
                      <span class="badge conflictbadge">!</span>
                      <span class="row-label">{path}</span>
                    </button>
                  </div>
                {/each}
              {/if}

              {#if status.staged.length > 0}
                <div class="group-title">
                  Staged <span class="count">{status.staged.length}</span>
                  <button class="mini link" onclick={() => act(() => gitUnstageAll(repo!.path)).then(clearDiff)}>
                    unstage all
                  </button>
                </div>
                {#each status.staged as file ("staged:" + file.path)}
                  <div class="row" class:selected={selectedKey === `staged:${file.path}`}>
                    <button class="row-main" onclick={() => selectFile(file.path, "staged")}>
                      <span class="badge {file.kind}">{kindBadge(file)}</span>
                      <span class="row-label" title={file.path}>
                        {#if file.origPath}{file.origPath} → {/if}{file.path}
                      </span>
                    </button>
                    <span class="row-actions">
                      <button class="mini" title="Unstage" onclick={() => unstageFile(file.path)}>
                        −
                      </button>
                    </span>
                  </div>
                {/each}
              {/if}

              {#if status.unstaged.length > 0}
                <div class="group-title">
                  Unstaged <span class="count">{status.unstaged.length}</span>
                </div>
                {#each status.unstaged as file ("worktree:" + file.path)}
                  <div class="row" class:selected={selectedKey === `worktree:${file.path}`}>
                    <button class="row-main" onclick={() => selectFile(file.path, "worktree")}>
                      <span class="badge {file.kind}">{kindBadge(file)}</span>
                      <span class="row-label" title={file.path}>
                        {#if file.origPath}{file.origPath} → {/if}{file.path}
                      </span>
                    </button>
                    <span class="row-actions">
                      <button class="mini" title="Stage" onclick={() => stageFile(file.path)}>+</button>
                      <button
                        class="mini danger"
                        title="Discard changes"
                        onclick={() => discardFile(file.path, false)}
                      >
                        ×
                      </button>
                    </span>
                  </div>
                {/each}
              {/if}

              {#if status.untracked.length > 0}
                <div class="group-title">
                  Untracked <span class="count">{status.untracked.length}</span>
                  <button class="mini link" onclick={() => act(() => gitStageAll(repo!.path)).then(clearDiff)}>
                    stage all
                  </button>
                </div>
                {#each status.untracked as path (path)}
                  <div class="row" class:selected={selectedKey === `untracked:${path}`}>
                    <button class="row-main" onclick={() => selectFile(path, "untracked")}>
                      <span class="badge added">?</span>
                      <span class="row-label" title={path}>{path}</span>
                    </button>
                    <span class="row-actions">
                      <button class="mini" title="Stage" onclick={() => stageFile(path)}>+</button>
                      <button
                        class="mini danger"
                        title="Delete file"
                        onclick={() => discardFile(path, true)}
                      >
                        ×
                      </button>
                    </span>
                  </div>
                {/each}
              {/if}

              {#if changeCount === 0}
                <div class="empty-note">Working tree clean</div>
              {/if}
            {/if}
          </div>

          <div class="commit-box">
            <div class="subject-wrap">
              <input
                class="subject"
                placeholder="Commit subject"
                bind:value={commitSubject}
                onkeydown={(e) => {
                  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") doCommit();
                }}
              />
              <span
                class="subject-count"
                class:over={commitSubject.length > 72}
                title="Subject length; 50 is conventional, 72 the hard ceiling"
              >
                {commitSubject.length || ""}
              </span>
            </div>
            <textarea
              placeholder="Body (optional)"
              bind:value={commitBody}
              rows="3"
              onkeydown={(e) => {
                if ((e.metaKey || e.ctrlKey) && e.key === "Enter") doCommit();
              }}
            ></textarea>
            <div class="commit-actions">
              <label class="amend">
                <input type="checkbox" bind:checked={amend} />
                Amend
              </label>
              <button class="primary" disabled={!canCommit} onclick={doCommit}>
                Commit{status && status.staged.length > 0 ? ` (${status.staged.length})` : ""}
              </button>
            </div>
          </div>
        {:else}
          <div class="scroll">
            {#each commits as commit (commit.hash)}
              <div class="row commit" class:selected={selectedKey === commit.hash}>
                <button class="row-main commit-main" onclick={() => selectCommit(commit)}>
                  <Avatar email={commit.email} name={commit.author} size={24} />
                  <span class="commit-text">
                    <span class="commit-subject" title={commit.subject}>{commit.subject}</span>
                    <span class="commit-meta">
                      <code>{commit.shortHash}</code>
                      {commit.author} · {commitDate(commit.date)}
                      {#if commit.parents.length > 1}· merge{/if}
                    </span>
                  </span>
                </button>
              </div>
            {:else}
              <div class="empty-note">No commits yet</div>
            {/each}
            {#if commits.length > 0 && !logExhausted}
              <button class="load-more" disabled={loadingMore} onclick={loadMore}>
                {loadingMore ? "Loading…" : "Load more"}
              </button>
            {/if}
          </div>
        {/if}
      </section>

      <section class="diff-pane">
        {#if diffTitle}
          <div class="diff-title" title={diffTitle}>{diffTitle}</div>
        {/if}
        {#if details}
          <div class="details">
            <div class="details-head">
              <Avatar email={details.author.email} name={details.author.name} size={40} />
              <div class="details-who">
                <span class="details-name">{details.author.name}</span>
                <a class="details-email" href="mailto:{details.author.email}">
                  {details.author.email}
                </a>
                <span class="details-date">{fullDate(details.author.date)}</span>
              </div>
              <div class="details-ids">
                <code title={details.hash}>{details.shortHash}</code>
                {#if details.parents.length > 1}<span class="merge-tag">merge</span>{/if}
              </div>
            </div>
            {#if details.committer.email !== details.author.email || details.committer.name !== details.author.name}
              <div class="details-committer">
                <Avatar email={details.committer.email} name={details.committer.name} size={16} />
                committed by {details.committer.name}
                &lt;{details.committer.email}&gt; · {fullDate(details.committer.date)}
              </div>
            {/if}
            {#if details.message.includes("\n")}
              <pre class="details-message">{details.message}</pre>
            {/if}
            {#if details.files.length > 0}
              <div class="details-files">
                {#each details.files as file (file.path)}
                  <span class="file-chip">
                    <span class="badge {file.kind}">{kindBadge(file)}</span>
                    {#if file.origPath}{file.origPath} → {/if}{file.path}
                  </span>
                {/each}
              </div>
            {/if}
          </div>
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
    grid-template-columns: 210px 330px 1fr;
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
    display: flex;
    align-items: center;
    gap: 0.4rem;
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
    width: 100%;
    box-sizing: border-box;
  }
  .row:hover {
    background: var(--hover, #161b22);
  }
  .row.selected {
    background: var(--selected, #1f2937);
  }
  .row.head {
    font-weight: 600;
  }
  .row.remote {
    color: var(--fg-muted, #8b949e);
  }
  .row-main {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
    text-align: left;
    background: none;
    border: none;
    color: inherit;
    padding: 0.3rem 0.75rem;
    font-size: 13px;
    cursor: pointer;
  }
  .row-main:disabled {
    cursor: default;
  }
  .row-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .static-label {
    padding: 0.3rem 0.75rem;
    font-size: 13px;
  }
  .row-actions {
    display: none;
    gap: 0.2rem;
    padding-right: 0.5rem;
    flex: none;
  }
  .row:hover .row-actions {
    display: flex;
  }
  .track {
    font-size: 11px;
    color: var(--fg-muted, #8b949e);
    flex: none;
  }

  .mini {
    background: var(--chip, #21262d);
    color: var(--fg, #e6edf3);
    border: 1px solid var(--border, #30363d);
    border-radius: 4px;
    font-size: 12px;
    line-height: 1;
    padding: 0.15rem 0.45rem;
    cursor: pointer;
  }
  .mini.danger:hover {
    background: rgba(248, 81, 73, 0.2);
    border-color: #f85149;
  }
  .mini.link {
    background: none;
    border: none;
    color: var(--accent, #58a6ff);
    text-transform: none;
    letter-spacing: normal;
    padding: 0;
  }
  .mini:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .new-branch {
    display: flex;
    gap: 0.3rem;
    padding: 0.4rem 0.75rem;
  }
  .new-branch input {
    flex: 1;
    min-width: 0;
    background: var(--bg, #0d1117);
    border: 1px solid var(--border, #30363d);
    border-radius: 4px;
    color: var(--fg, #e6edf3);
    padding: 0.2rem 0.4rem;
    font-size: 12px;
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

  .recent {
    margin-top: 1.5rem;
    width: 420px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
  }
  .recent-row {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    background: none;
    border: none;
    border-radius: 6px;
    color: var(--fg, #e6edf3);
    padding: 0.4rem 0.75rem;
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }
  .recent-row:hover {
    background: var(--hover, #161b22);
  }
  .recent-name {
    font-weight: 600;
    flex: none;
  }
  .recent-path {
    color: var(--fg-muted, #8b949e);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row.commit {
    padding: 0;
  }
  .commit-main {
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.75rem;
  }
  .commit-text {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
    flex: 1;
  }
  .load-more {
    display: block;
    margin: 0.5rem auto 0.75rem;
    background: var(--chip, #21262d);
    color: var(--fg, #e6edf3);
    border: 1px solid var(--border, #30363d);
    border-radius: 6px;
    padding: 0.3rem 1rem;
    font-size: 12px;
    cursor: pointer;
  }
  .load-more:disabled {
    opacity: 0.5;
    cursor: default;
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

  .commit-box {
    border-top: 1px solid var(--border, #30363d);
    padding: 0.6rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    flex: none;
  }
  .commit-box textarea {
    resize: vertical;
    min-height: 3.2em;
    background: var(--bg, #0d1117);
    border: 1px solid var(--border, #30363d);
    border-radius: 6px;
    color: var(--fg, #e6edf3);
    padding: 0.4rem 0.6rem;
    font-size: 13px;
    font-family: inherit;
  }
  .subject-wrap {
    position: relative;
    display: flex;
  }
  .subject {
    flex: 1;
    background: var(--bg, #0d1117);
    border: 1px solid var(--border, #30363d);
    border-radius: 6px;
    color: var(--fg, #e6edf3);
    padding: 0.4rem 2.6rem 0.4rem 0.6rem;
    font-size: 13px;
  }
  .subject-count {
    position: absolute;
    right: 0.6rem;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11px;
    color: var(--fg-muted, #8b949e);
  }
  .subject-count.over {
    color: #f85149;
  }
  .commit-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .amend {
    font-size: 12px;
    color: var(--fg-muted, #8b949e);
    display: flex;
    align-items: center;
    gap: 0.3rem;
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

  .details {
    border-bottom: 1px solid var(--border, #30363d);
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    flex: none;
    max-height: 40%;
    overflow-y: auto;
  }
  .details-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .details-who {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
    flex: 1;
  }
  .details-name {
    font-weight: 600;
    font-size: 14px;
  }
  .details-email {
    color: var(--accent, #58a6ff);
    font-size: 12px;
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .details-email:hover {
    text-decoration: underline;
  }
  .details-date {
    color: var(--fg-muted, #8b949e);
    font-size: 12px;
  }
  .details-ids {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex: none;
  }
  .details-ids code {
    color: var(--accent, #58a6ff);
    font-size: 12px;
    background: var(--chip, #21262d);
    border-radius: 4px;
    padding: 0.15rem 0.4rem;
  }
  .merge-tag {
    font-size: 11px;
    color: var(--fg-muted, #8b949e);
    border: 1px solid var(--border, #30363d);
    border-radius: 999px;
    padding: 0.05rem 0.5rem;
  }
  .details-committer {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--fg-muted, #8b949e);
    font-size: 12px;
  }
  .details-message {
    margin: 0;
    font-family: inherit;
    font-size: 13px;
    white-space: pre-wrap;
    color: var(--fg, #e6edf3);
    background: var(--chip, #21262d);
    border-radius: 6px;
    padding: 0.5rem 0.6rem;
  }
  .details-files {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .file-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 11px;
    font-family: ui-monospace, monospace;
    background: var(--chip, #21262d);
    border: 1px solid var(--border, #30363d);
    border-radius: 999px;
    padding: 0.1rem 0.5rem;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  input,
  textarea {
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
  .toolbar button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  button.primary {
    background: var(--accent-bg, #238636);
    border-color: transparent;
    font-size: 14px;
  }
  button.primary:hover:not(:disabled) {
    background: #2ea043;
  }
  button.primary:disabled {
    opacity: 0.5;
    cursor: default;
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
