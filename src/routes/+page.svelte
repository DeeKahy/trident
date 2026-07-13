<script lang="ts">
  import { confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
  import { langColor } from "$lib/langColors";
  import { listen } from "@tauri-apps/api/event";
  import DiffView from "$lib/DiffView.svelte";
  import Avatar from "$lib/Avatar.svelte";
  import Hub from "$lib/Hub.svelte";
  import {
    openRepo,
    gitStatus,
    gitLog,
    gitBranches,
    gitDiffFile,
    gitCommitDiff,
    gitCommitDetails,
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
    gitTags,
    gitCreateTag,
    gitStashList,
    gitStashAll,
    gitAddIgnore,
    gitRemoveIgnore,
    gitUpdateMerge,
    gitUpdateRebase,
    gitPublishBranch,
    gitRewordHead,
    gitUndoLast,
    gitRevert,
    gitSwitchDetached,
    gitUntrackedLines,
    watchRepo,
    codeStats,
    type CodeStats,
    errorMessage,
    type RepoInfo,
    type Status,
    type CommitInfo,
    type CommitDetails,
    type BranchInfo,
    type TagInfo,
    type DiffMode,
    type ChangeKind,
  } from "$lib/git";

  // ---------- state ----------
  let repo = $state<RepoInfo | null>(null);
  let status = $state<Status | null>(null);
  let commits = $state<CommitInfo[]>([]);
  let branches = $state<BranchInfo[]>([]);
  let tags = $state<TagInfo[]>([]);
  let stashes = $state<string[]>([]);
  let untrackedCounts = $state<Record<string, number>>({});

  let error = $state<string | null>(null);
  let toast = $state<string | null>(null);

  let railExpanded = $state(false);
  let branchMenu = $state(false);
  let syncMenu = $state(false);
  let centerMode = $state<"history" | "releases" | "stats">("history");
  let scope = $state<"all" | "current">("current");
  let stats = $state<CodeStats | null>(null);
  let statsLoading = $state(false);
  let logExhausted = $state(false);
  let loadingMore = $state(false);

  let rightView = $state<"working" | "detail">("working");
  let selectedPath = $state<string | null>(null);
  let diffText = $state("");
  let selectedCommit = $state<CommitInfo | null>(null);
  let details = $state<CommitDetails | null>(null);
  let commitDiffText = $state("");
  let editedMessage = $state<string | null>(null);

  let summary = $state("");
  let mainDismissed = $state(false);
  let bannerBranchName = $state("");
  let bannerNaming = $state(false);
  let newBranchName = $state("");
  let newBranchOpen = $state(false);
  let ignoredSession = $state<string[]>([]);
  let busy = $state<string | null>(null);

  let diffModal = $state(false);
  let diffModalTitle = $state("");
  let diffModalText = $state("");
  // How to re-fetch the open modal's diff at a given context (for the "Full
  // file" toggle); null when the source can't be reloaded.
  let diffModalReload = $state<((full: boolean) => Promise<string>) | null>(null);
  let diffModalFull = $state(false);
  let diffModalLoading = $state(false);

  let releaseModal = $state(false);
  let version = $state("0.1.0");
  let relTitle = $state("");
  let relNotes = $state("");
  let pushTag = $state(true);

  const LOG_PAGE = 200;
  const RECENT_KEY = "trident.recentRepos";

  // ---------- resizable panes ----------
  const RIGHT_W_KEY = "trident.rightPaneWidth";
  const FILES_H_KEY = "trident.fileListHeight";

  function storedInt(key: string, fallback: number): number {
    const n = parseInt(localStorage.getItem(key) ?? "");
    return Number.isFinite(n) ? n : fallback;
  }

  let rightWidth = $state(storedInt(RIGHT_W_KEY, 472));
  let fileListHeight = $state(storedInt(FILES_H_KEY, 150));
  let panesEl = $state<HTMLDivElement | null>(null);

  /// Generic pointer-drag helper for the splitters: calls `apply` with each
  /// pointer position and persists via `save` when the drag ends.
  function dragSplitter(down: PointerEvent, apply: (ev: PointerEvent) => void, save: () => void) {
    down.preventDefault();
    document.body.style.userSelect = "none";
    const move = (ev: PointerEvent) => apply(ev);
    const up = () => {
      document.body.style.userSelect = "";
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      save();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function startColDrag(e: PointerEvent) {
    dragSplitter(
      e,
      (ev) => {
        if (!panesEl) return;
        const rect = panesEl.getBoundingClientRect();
        // History keeps at least 380px; the changes pane at least 360px.
        const max = rect.width - 54 - 380;
        rightWidth = Math.round(Math.min(Math.max(rect.right - ev.clientX, 360), max));
      },
      () => localStorage.setItem(RIGHT_W_KEY, String(rightWidth))
    );
  }

  function startRowDrag(e: PointerEvent) {
    const top = (e.currentTarget as HTMLElement).parentElement
      ?.querySelector(".file-list")
      ?.getBoundingClientRect().top;
    dragSplitter(
      e,
      (ev) => {
        if (top === undefined) return;
        fileListHeight = Math.round(Math.min(Math.max(ev.clientY - top, 60), 500));
      },
      () => localStorage.setItem(FILES_H_KEY, String(fileListHeight))
    );
  }

  // ---------- derived ----------
  const PALETTE = ["#e2683c", "#2f8f5b", "#9b59b6", "#0f8f6b", "#d9534f", "#b8860b", "#5b3df5"];

  let localBranches = $derived(branches.filter((b) => !b.isRemote));
  let colorOf = $derived.by(() => {
    const map = new Map<string, string>();
    let i = 0;
    for (const b of localBranches) {
      map.set(b.name, b.name === "main" || b.name === "master" ? "#2f5fe0" : PALETTE[i++ % PALETTE.length]);
    }
    return (name: string) => map.get(name) ?? "#2f5fe0";
  });
  let currentColor = $derived(status ? colorOf(status.branch.head) : "#2f5fe0");
  let remoteOnly = $derived(
    branches.filter(
      (b) =>
        b.isRemote &&
        !localBranches.some((l) => b.name === `origin/${l.name}` || b.name.endsWith(`/${l.name}`))
    )
  );
  let branchTips = $derived.by(() => {
    const map = new Map<string, { name: string; color: string }>();
    // remotes first so local tips win when both point at the same commit
    for (const b of branches.filter((x) => x.isRemote)) {
      map.set(b.shortHash, { name: b.name, color: "#8f877b" });
    }
    for (const b of localBranches) {
      map.set(b.shortHash, { name: b.name, color: colorOf(b.name) });
    }
    return map;
  });

  interface ChangeRow {
    path: string;
    kind: ChangeKind | "conflict";
    checked: boolean;
    untracked: boolean;
    add: number;
    del: number;
    /// Set when the row stands in for a whole directory of changes
    /// (build output, node_modules, and similar floods).
    bundleCount?: number;
  }

  const BUNDLE_THRESHOLD = 50;

  /// Top-level directories with so many changed files that listing them
  /// individually would freeze the UI (22k untracked files in target/).
  let bundledDirs = $derived.by(() => {
    if (!status) return new Set<string>();
    const counts = new Map<string, number>();
    const bump = (p: string) => {
      const i = p.indexOf("/");
      if (i > 0) {
        const dir = p.slice(0, i);
        counts.set(dir, (counts.get(dir) ?? 0) + 1);
      }
    };
    for (const f of status.staged) bump(f.path);
    for (const f of status.unstaged) bump(f.path);
    for (const p of status.untracked) bump(p);
    return new Set([...counts].filter(([, n]) => n > BUNDLE_THRESHOLD).map(([d]) => d));
  });
  const bundleOf = (path: string): string | null => {
    const i = path.indexOf("/");
    if (i <= 0) return null;
    const dir = path.slice(0, i);
    return bundledDirs.has(dir) ? dir : null;
  };

  let changeRows = $derived.by<ChangeRow[]>(() => {
    if (!status) return [];
    const rows = new Map<string, ChangeRow>();
    const row = (path: string): ChangeRow => {
      if (!rows.has(path)) {
        rows.set(path, { path, kind: "modified", checked: false, untracked: false, add: 0, del: 0 });
      }
      return rows.get(path)!;
    };
    interface Bundle {
      paths: Set<string>;
      dirty: Set<string>; // unstaged or untracked
      staged: Set<string>;
      untrackedOnly: boolean;
      add: number;
      del: number;
    }
    const bundles = new Map<string, Bundle>();
    const bundle = (dir: string): Bundle => {
      if (!bundles.has(dir)) {
        bundles.set(dir, {
          paths: new Set(),
          dirty: new Set(),
          staged: new Set(),
          untrackedOnly: true,
          add: 0,
          del: 0,
        });
      }
      return bundles.get(dir)!;
    };

    for (const f of status.staged) {
      const dir = bundleOf(f.path);
      if (dir) {
        const b = bundle(dir);
        b.paths.add(f.path);
        b.staged.add(f.path);
        b.untrackedOnly = false;
        b.add += f.additions;
        b.del += f.deletions;
        continue;
      }
      const r = row(f.path);
      r.kind = f.kind;
      r.checked = true;
      r.add += f.additions;
      r.del += f.deletions;
    }
    for (const f of status.unstaged) {
      const dir = bundleOf(f.path);
      if (dir) {
        const b = bundle(dir);
        b.paths.add(f.path);
        b.dirty.add(f.path);
        b.untrackedOnly = false;
        b.add += f.additions;
        b.del += f.deletions;
        continue;
      }
      const r = row(f.path);
      if (!r.checked) r.kind = f.kind;
      r.add += f.additions;
      r.del += f.deletions;
    }
    for (const p of status.untracked) {
      const dir = bundleOf(p);
      if (dir) {
        const b = bundle(dir);
        b.paths.add(p);
        b.dirty.add(p);
        continue;
      }
      const r = row(p);
      r.kind = "added";
      r.untracked = true;
      r.add = untrackedCounts[p] ?? 0;
    }
    for (const p of status.conflicted) {
      if (bundleOf(p)) continue;
      row(p).kind = "conflict";
    }

    const result = [...rows.values()];
    for (const [dir, b] of bundles) {
      result.push({
        path: `${dir}/`,
        kind: b.untrackedOnly ? "added" : "modified",
        checked: b.paths.size > 0 && b.dirty.size === 0,
        untracked: b.untrackedOnly,
        add: b.add,
        del: b.del,
        bundleCount: b.paths.size,
      });
    }
    return result;
  });
  let checkedCount = $derived(changeRows.filter((r) => r.checked).length);
  let allChecked = $derived(changeRows.length > 0 && checkedCount === changeRows.length);
  let someChecked = $derived(checkedCount > 0 && !allChecked);
  let canCommit = $derived(summary.trim().length > 0 && checkedCount > 0);
  let onMain = $derived(
    !!status &&
      (status.branch.head === "main" || status.branch.head === "master") &&
      changeRows.length > 0 &&
      !mainDismissed
  );
  let selectedRow = $derived(changeRows.find((r) => r.path === selectedPath) ?? null);
  let latestTag = $derived(tags.length > 0 ? tags[0] : null);
  let selectedIsHead = $derived(
    !!selectedCommit && !!status && status.branch.oid !== "" && selectedCommit.hash.startsWith(status.branch.oid)
  );
  let selectedEditable = $derived(!!selectedCommit && selectedCommit.localOnly && selectedIsHead);

  // ---------- helpers ----------
  /// Keep the hub's project list up to date: newest first, capped, deduped.
  function rememberRepo(info: RepoInfo) {
    let list: { path: string; name: string }[] = [];
    try {
      list = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    } catch {
      list = [];
    }
    list = [{ path: info.path, name: info.name }, ...list.filter((r) => r.path !== info.path)].slice(0, 24);
    localStorage.setItem(RECENT_KEY, JSON.stringify(list));
  }

  function relTime(iso: string): string {
    const s = (Date.now() - new Date(iso).getTime()) / 1000;
    if (s < 90) return "just now";
    if (s < 3600) return `${Math.round(s / 60)}m ago`;
    if (s < 86400) return `${Math.round(s / 3600)}h ago`;
    if (s < 604800) return `${Math.round(s / 86400)}d ago`;
    return new Date(iso).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  }

  function badgeOf(kind: ChangeRow["kind"]): string {
    return (
      { modified: "M", added: "A", deleted: "D", renamed: "R", copied: "C", typechange: "T", conflict: "!" }[kind] ??
      "M"
    );
  }

  function showToast(text: string) {
    toast = text;
    setTimeout(() => {
      if (toast === text) toast = null;
    }, 3200);
  }

  // ---------- data flow ----------
  async function openPath(path: string) {
    error = null;
    try {
      repo = await openRepo(path);
      rememberRepo(repo);
      rightView = "working";
      selectedPath = null;
      selectedCommit = null;
      commits = [];
      logExhausted = false;
      mainDismissed = false;
      stats = null;
      ignoredSession = [];
      await refresh();
      await watchRepo(repo.path);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function refresh() {
    if (!repo) return;
    try {
      const limit = Math.max(commits.length, LOG_PAGE);
      const [s, c, b, t, st] = await Promise.all([
        gitStatus(repo.path),
        gitLog(repo.path, limit, 0, scope === "all"),
        gitBranches(repo.path),
        gitTags(repo.path),
        gitStashList(repo.path),
      ]);
      status = s;
      commits = c;
      branches = b;
      tags = t;
      stashes = st;
      const counts: Record<string, number> = {};
      await Promise.all(
        s.untracked
          .filter((p) => !bundleOf(p))
          .slice(0, 100)
          .map(async (p) => {
            counts[p] = await gitUntrackedLines(repo!.path, p);
          })
      );
      untrackedCounts = counts;
      await resyncDiff();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function resyncDiff() {
    if (!repo || !status) return;
    // keep a valid file selected in the working view
    if (rightView === "working") {
      if (!selectedPath || !changeRows.some((r) => r.path === selectedPath)) {
        selectedPath = changeRows[0]?.path ?? null;
      }
      if (selectedPath) {
        await loadDiff(selectedPath);
      } else {
        diffText = "";
      }
    }
  }

  async function loadDiff(path: string) {
    if (!repo) return;
    const row = changeRows.find((r) => r.path === path);
    if (!row) return;
    if (row.bundleCount) {
      // A whole directory of changes: nothing sensible to preview.
      diffText = "";
      return;
    }
    const mode: DiffMode = row.untracked
      ? "untracked"
      : status!.unstaged.some((f) => f.path === path)
        ? "worktree"
        : "staged";
    try {
      diffText = await gitDiffFile(repo.path, path, mode);
    } catch (e) {
      diffText = "";
      error = errorMessage(e);
    }
  }

  async function act(fn: () => Promise<unknown>, okToast?: string) {
    if (!repo) return;
    error = null;
    try {
      await fn();
      if (okToast) showToast(okToast);
    } catch (e) {
      error = errorMessage(e);
    }
    await refresh();
  }

  // auto-refresh from the fs watcher, debounced
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

  // ---------- changes ----------
  function selectRow(path: string) {
    selectedPath = path;
    loadDiff(path);
  }

  async function toggleRow(row: ChangeRow) {
    await act(() =>
      row.checked ? gitUnstageFile(repo!.path, row.path) : gitStageFile(repo!.path, row.path)
    );
  }

  async function toggleAll() {
    await act(() => (allChecked ? gitUnstageAll(repo!.path) : gitStageAll(repo!.path)));
  }

  async function ignoreRow(path: string) {
    await act(() => gitAddIgnore(repo!.path, path), `${path} added to .gitignore`);
    if (!error) ignoredSession = [...ignoredSession, path];
  }

  async function undoIgnore(path: string) {
    await act(() => gitRemoveIgnore(repo!.path, path));
    ignoredSession = ignoredSession.filter((p) => p !== path);
  }

  async function discardRow(row: ChangeRow) {
    const verb = row.untracked ? "Delete untracked file" : "Discard changes to";
    const ok = await confirmDialog(`${verb} "${row.path}"? This cannot be undone.`, {
      title: "Trident",
      kind: "warning",
    });
    if (!ok) return;
    await act(() => gitDiscardFile(repo!.path, row.path, row.untracked));
  }

  async function stashAll() {
    await act(() => gitStashAll(repo!.path, summary.trim()), "Changes stashed safely");
  }

  async function doCommit() {
    if (!canCommit) return;
    await act(() => gitCommit(repo!.path, summary.trim(), false), "Committed");
    if (!error) {
      summary = "";
      mainDismissed = false;
    }
  }

  async function branchFromBanner() {
    const name = bannerBranchName.trim();
    if (!name) return;
    await act(() => gitCreateBranch(repo!.path, name, true), `Now on ${name} - changes came along`);
    bannerNaming = false;
    bannerBranchName = "";
  }

  // ---------- branches ----------
  async function switchTo(b: BranchInfo) {
    if (b.isHead) return;
    branchMenu = false;
    await act(() => gitSwitchBranch(repo!.path, b.name));
    rightView = "working";
    mainDismissed = false;
  }

  async function checkoutRemote(b: BranchInfo) {
    const short = b.name.replace(/^[^/]+\//, "");
    await act(() => gitSwitchBranch(repo!.path, short), `Checked out ${short}`);
  }

  async function publish(b: BranchInfo) {
    await act(() => gitPublishBranch(repo!.path, b.name), `${b.name} published to origin`);
  }

  async function createBranch() {
    const name = newBranchName.trim();
    if (!name) return;
    await act(() => gitCreateBranch(repo!.path, name, true), `Created ${name}`);
    if (!error) {
      newBranchName = "";
      newBranchOpen = false;
      branchMenu = false;
    }
  }

  async function deleteBranchUI(b: BranchInfo) {
    const ok = await confirmDialog(`Delete branch "${b.name}"?`, { title: "Trident", kind: "warning" });
    if (!ok || !repo) return;
    try {
      await gitDeleteBranch(repo.path, b.name, false);
    } catch (e) {
      const message = errorMessage(e);
      if (message.includes("not fully merged")) {
        const force = await confirmDialog(
          `"${b.name}" has commits that exist nowhere else. Delete anyway and lose them?`,
          { title: "Trident", kind: "warning" }
        );
        if (force) await act(() => gitDeleteBranch(repo!.path, b.name, true));
      } else {
        error = message;
      }
    }
    await refresh();
  }

  // ---------- sync ----------
  async function run(label: string, fn: () => Promise<unknown>, okToast: string) {
    if (!repo || busy) return;
    busy = label;
    syncMenu = false;
    await act(fn, okToast);
    busy = null;
  }

  async function doSync() {
    if (!repo || busy || !status) return;
    busy = "sync";
    error = null;
    try {
      await gitFetch(repo.path);
      const s = await gitStatus(repo.path);
      if (s.branch.behind > 0 && s.branch.ahead > 0) {
        syncMenu = true; // diverged: the user picks rebase or merge
      } else if (s.branch.behind > 0) {
        await gitPull(repo.path);
        showToast(`Pulled ${s.branch.behind} new commit${s.branch.behind === 1 ? "" : "s"}`);
      } else if (s.branch.ahead > 0) {
        await gitPush(repo.path);
        showToast(`Pushed ${s.branch.ahead} commit${s.branch.ahead === 1 ? "" : "s"}`);
      } else {
        showToast("Up to date");
      }
    } catch (e) {
      error = errorMessage(e);
    }
    busy = null;
    await refresh();
  }

  // ---------- history ----------
  async function selectCommit(c: CommitInfo) {
    if (!repo) return;
    selectedCommit = c;
    rightView = "detail";
    editedMessage = null;
    details = null;
    commitDiffText = "";
    try {
      [details, commitDiffText] = await Promise.all([
        gitCommitDetails(repo.path, c.hash),
        gitCommitDiff(repo.path, c.hash),
      ]);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function backToWorking() {
    rightView = "working";
    selectedCommit = null;
    details = null;
    resyncDiff();
  }

  async function saveMessage() {
    if (!repo || editedMessage === null || !editedMessage.trim()) return;
    await act(() => gitRewordHead(repo!.path, editedMessage!), "Message rewritten");
    if (!error && commits.length > 0) {
      selectCommit(commits[0]);
    }
  }

  async function undoLast(keep: boolean) {
    if (!keep) {
      const ok = await confirmDialog(
        "Undo this commit and delete its changes from disk? This cannot be recovered.",
        { title: "Trident", kind: "warning" }
      );
      if (!ok) return;
    }
    await act(() => gitUndoLast(repo!.path, keep), keep ? "Commit undone - changes kept" : "Commit undone");
    backToWorking();
  }

  async function revertSelected() {
    if (!selectedCommit) return;
    await act(() => gitRevert(repo!.path, selectedCommit!.hash), "Revert commit created");
    backToWorking();
  }

  async function checkoutHere() {
    if (!selectedCommit) return;
    const ok = await confirmDialog(
      `Check out ${selectedCommit.shortHash} directly? You'll be "detached": looking at an old snapshot without being on a branch. Switch to any branch to get back.`,
      { title: "Trident" }
    );
    if (!ok) return;
    await act(() => gitSwitchDetached(repo!.path, selectedCommit!.hash));
    backToWorking();
  }

  async function setScope(next: "all" | "current") {
    scope = next;
    logExhausted = false;
    await refresh();
  }

  /// Recount on every visit; the previous numbers stay visible meanwhile.
  async function openStats() {
    centerMode = "stats";
    if (!repo || statsLoading) return;
    statsLoading = true;
    try {
      stats = await codeStats(repo.path);
    } catch (e) {
      error = errorMessage(e);
    }
    statsLoading = false;
  }

  function repoAge(iso: string): string {
    const days = (Date.now() - new Date(iso).getTime()) / 86400000;
    if (days < 1) return "today";
    if (days < 60) return `${Math.round(days)} days`;
    if (days < 700) return `${Math.round(days / 30.4)} months`;
    return `${(days / 365).toFixed(1)} years`;
  }

  async function loadMore() {
    if (!repo || loadingMore) return;
    loadingMore = true;
    try {
      const older = await gitLog(repo.path, LOG_PAGE, commits.length, scope === "all");
      commits = [...commits, ...older];
      if (older.length < LOG_PAGE) logExhausted = true;
    } catch (e) {
      error = errorMessage(e);
    }
    loadingMore = false;
  }

  // ---------- releases ----------
  function bumped(kind: "patch" | "minor" | "major"): string {
    const p = version.split(".").map((n) => parseInt(n) || 0);
    let [a, b, c] = [p[0] || 0, p[1] || 0, p[2] || 0];
    if (kind === "patch") c += 1;
    else if (kind === "minor") {
      b += 1;
      c = 0;
    } else {
      a += 1;
      b = 0;
      c = 0;
    }
    return `${a}.${b}.${c}`;
  }

  function openReleaseModal() {
    version = latestTag ? bumpFrom(latestTag.name) : "0.1.0";
    relTitle = "";
    relNotes = "";
    releaseModal = true;
  }
  function bumpFrom(tag: string): string {
    const clean = tag.replace(/^v/, "");
    const p = clean.split(".").map((n) => parseInt(n) || 0);
    return `${p[0] || 0}.${(p[1] || 0) + 1}.0`;
  }

  function generateNotes() {
    let since = commits;
    if (latestTag) {
      const idx = commits.findIndex((c) => c.hash.startsWith(latestTag!.hash));
      if (idx > 0) since = commits.slice(0, idx);
    }
    relNotes = since
      .slice(0, 20)
      .map((c) => `- ${c.subject}`)
      .join("\n");
  }

  async function createRelease() {
    const name = `v${version.trim()}`;
    const message = relTitle.trim()
      ? `${relTitle.trim()}\n\n${relNotes.trim()}`.trim()
      : relNotes.trim();
    await act(
      () => gitCreateTag(repo!.path, name, message, pushTag),
      `${name} created${pushTag ? " and pushed" : ""}`
    );
    if (!error) {
      releaseModal = false;
      centerMode = "releases";
    }
  }

  function openDiffModal(
    title: string,
    text: string,
    reload: ((full: boolean) => Promise<string>) | null = null
  ) {
    diffModalTitle = title;
    diffModalText = text;
    diffModalReload = reload;
    diffModalFull = false;
    diffModalLoading = false;
    diffModal = true;
  }

  // Swap the modal between changed-hunks-only and the whole file.
  async function toggleDiffFull() {
    if (!diffModalReload || diffModalLoading) return;
    const next = !diffModalFull;
    diffModalLoading = true;
    try {
      diffModalText = await diffModalReload(next);
      diffModalFull = next;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      diffModalLoading = false;
    }
  }

  // Which diff mode a working-tree path is shown in (mirrors loadDiff).
  function diffModeFor(path: string): DiffMode {
    const row = changeRows.find((r) => r.path === path);
    if (row?.untracked) return "untracked";
    return status?.unstaged.some((f) => f.path === path) ? "worktree" : "staged";
  }
</script>

<!-- ==================== PROJECT HUB ==================== -->
{#if repo === null}
  <Hub onopen={openPath} />
{:else}
  <!-- ==================== WORKSPACE ==================== -->
  <main class="app">
    <!-- toolbar -->
    <header class="toolbar">
      <button class="back mono" onclick={() => (repo = null)}>← projects</button>
      <div class="repo-badge">
        <div class="repo-icon">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round"><circle cx="6" cy="6" r="2.4" /><circle cx="6" cy="18" r="2.4" /><circle cx="18" cy="9" r="2.4" /><path d="M6 8.4v7.2M8.2 6.6c6 0 7.6 1 7.6 4.2" /></svg>
        </div>
        <div>
          <div class="repo-name">{repo.name}</div>
          <div class="repo-path mono">{repo.path}</div>
        </div>
      </div>

      <!-- branch switcher -->
      <div class="dropdown-anchor">
        <button class="branch-btn" onclick={() => (branchMenu = !branchMenu)}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke={currentColor} stroke-width="2.2" stroke-linecap="round"><circle cx="6" cy="6" r="2.4" /><circle cx="6" cy="18" r="2.4" /><circle cx="18" cy="8" r="2.4" /><path d="M6 8.4v7.2M8.2 6.4c7 0 7.8 1.6 7.8 4" /></svg>
          <span class="branch-name">{status?.branch.head ?? repo.head}</span>
          {#if status?.branch.upstream}
            <span class="pill clean mono">published</span>
          {:else if branches.some((b) => b.isRemote)}
            <span class="pill muted mono">local</span>
          {/if}
          {#if status && (status.branch.ahead > 0 || status.branch.behind > 0)}
            <span class="mono updown">
              {#if status.branch.ahead > 0}↑{status.branch.ahead}{/if}
              {#if status.branch.behind > 0}↓{status.branch.behind}{/if}
            </span>
          {/if}
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="var(--muted)" stroke-width="2.6" stroke-linecap="round"><path d="M6 9l6 6 6-6" /></svg>
        </button>
        {#if branchMenu}
          <button class="scrim" onclick={() => (branchMenu = false)} aria-label="close"></button>
          <div class="menu">
            <div class="menu-label mono">SWITCH BRANCH</div>
            {#each localBranches as b (b.name)}
              <button class="menu-row" onclick={() => switchTo(b)}>
                <span class="dot" style="background:{colorOf(b.name)}"></span>
                <span class="menu-row-name">{b.name}</span>
                {#if b.isHead}<span class="mono tiny accent">current</span>{/if}
                {#if !b.upstream}<span class="mono tiny muted-text">local</span>{/if}
              </button>
            {/each}
            <div class="menu-sep"></div>
            {#if newBranchOpen}
              <form
                class="new-branch-form"
                onsubmit={(e) => {
                  e.preventDefault();
                  createBranch();
                }}
              >
                <input class="mono" placeholder="branch-name" bind:value={newBranchName} />
                <button type="submit" class="btn-accent small">Create</button>
              </form>
            {:else}
              <button class="menu-row accent-text" onclick={() => (newBranchOpen = true)}>+ New branch from here</button>
            {/if}
          </div>
        {/if}
      </div>

      <span class="spacer"></span>

      <button class="btn" onclick={openReleaseModal}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="M20.6 13.4l-7.2 7.2a2 2 0 01-2.8 0l-7-7a2 2 0 01-.6-1.4V4a1 1 0 011-1h8.2a2 2 0 011.4.6l7 7a2 2 0 010 2.8z" /><circle cx="7.5" cy="7.5" r="1.3" fill="currentColor" /></svg>
        Release
      </button>
      <div class="dropdown-anchor sync-split">
        <button class="btn-accent sync-main" onclick={doSync} disabled={busy !== null}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 01-9 9 9 9 0 01-6.7-3M3 12a9 9 0 019-9 9 9 0 016.7 3M3 4v5h5M21 20v-5h-5" /></svg>
          {busy ? "Working…" : "Sync"}
        </button>
        <button class="btn-accent sync-caret" onclick={() => (syncMenu = !syncMenu)} disabled={busy !== null} aria-label="sync options">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.6" stroke-linecap="round"><path d="M6 9l6 6 6-6" /></svg>
        </button>
        {#if syncMenu}
          <button class="scrim" onclick={() => (syncMenu = false)} aria-label="close"></button>
          <div class="menu wide right">
            <button class="menu-card" onclick={() => run("rebase", () => gitUpdateRebase(repo!.path), "Updated with rebase")}>
              <span class="menu-card-icon">↻</span>
              <span class="menu-card-body">
                <span class="menu-card-title">Update with rebase <span class="pill clean mono tiny-pill">TIDY</span></span>
                <span class="menu-card-sub mono">Replays your commits on top of the latest upstream. No extra merge commit, straight-line history.</span>
              </span>
            </button>
            <button class="menu-card" onclick={() => run("merge", () => gitUpdateMerge(repo!.path), "Updated with merge")}>
              <span class="menu-card-icon">⇄</span>
              <span class="menu-card-body">
                <span class="menu-card-title">Update with merge</span>
                <span class="menu-card-sub mono">Combines upstream into your branch, adding one merge commit. Safe and simple.</span>
              </span>
            </button>
            <div class="menu-sep"></div>
            <button class="menu-row" onclick={() => run("fetch", () => gitFetch(repo!.path), "Fetched")}>
              <span class="mr-icon">↓</span> Fetch only <span class="mono tiny muted-text pushed-right">check, don't apply</span>
            </button>
            <button class="menu-row" onclick={() => run("push", () => gitPush(repo!.path), "Pushed")}>
              <span class="mr-icon">↑</span> Push only <span class="mono tiny muted-text pushed-right">send yours up</span>
            </button>
          </div>
        {/if}
      </div>
    </header>

    {#if error}
      <div class="banner error-banner">
        {error}
        <button class="banner-close" onclick={() => (error = null)}>×</button>
      </div>
    {/if}
    {#if toast}
      <div class="banner toast-banner">{toast}</div>
    {/if}

    <!-- three panes -->
    <div class="panes" bind:this={panesEl}>
      <!-- LEFT: branch rail -->
      <nav
        class="rail"
        class:expanded={railExpanded}
        onmouseenter={() => (railExpanded = true)}
        onmouseleave={() => (railExpanded = false)}
      >
        {#if !railExpanded}
          <div class="rail-collapsed">
            <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="var(--muted)" stroke-width="2.1" stroke-linecap="round"><circle cx="6" cy="6" r="2.4" /><circle cx="6" cy="18" r="2.4" /><circle cx="18" cy="8" r="2.4" /><path d="M6 8.4v7.2M8.2 6.4c7 0 7.8 1.6 7.8 4" /></svg>
            <div class="rail-dots">
              {#each localBranches as b (b.name)}
                <button
                  class="rail-dot"
                  title={b.name}
                  style="background:{colorOf(b.name)};box-shadow:{b.isHead ? '0 0 0 3px var(--accent-soft)' : 'none'}"
                  onclick={() => switchTo(b)}
                  aria-label={b.name}
                ></button>
              {/each}
            </div>
            <div class="rail-vertical mono">BRANCHES ›</div>
          </div>
        {:else}
          <div class="rail-expanded">
            <div class="rail-head">
              <span class="mono label">BRANCHES</span>
              <button class="plus-btn" title="New branch" onclick={() => { branchMenu = true; newBranchOpen = true; }}>+</button>
            </div>
            <div class="rail-scroll">
              <div class="mono section-label">◉ LOCAL · ON THIS MACHINE</div>
              {#each localBranches as b (b.name)}
                <div class="rail-row" class:current={b.isHead}>
                  <button class="rail-row-main" onclick={() => switchTo(b)}>
                    <span class="dot" style="background:{colorOf(b.name)}"></span>
                    <span class="rail-row-name">{b.name}</span>
                  </button>
                  {#if b.upstream && b.ahead > 0}
                    <span class="mono tiny clean-text" title="{b.ahead} commit(s) ready to push">↑{b.ahead}</span>
                  {:else if !b.upstream && branches.some((x) => x.isRemote)}
                    <button class="pill-btn mono" onclick={() => publish(b)}>Publish</button>
                  {/if}
                  {#if !b.isHead}
                    <button class="x-btn" title="Delete branch" onclick={() => deleteBranchUI(b)}>×</button>
                  {/if}
                </div>
              {/each}
              {#if remoteOnly.length > 0}
                <div class="mono section-label">☁ REMOTE · ON ORIGIN ONLY</div>
                {#each remoteOnly as b (b.name)}
                  <div class="rail-row remote">
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="var(--muted)" stroke-width="2" style="flex:none"><path d="M18 10a4 4 0 000-8 5 5 0 00-9.6 1.5A4.5 4.5 0 004 12" /><path d="M6 16h12a3 3 0 000-6H6a3 3 0 000 6z" fill="var(--surface)" /></svg>
                    <span class="rail-row-name muted-text">{b.name}</span>
                    <button class="pill-btn outline mono" onclick={() => checkoutRemote(b)}>Check out</button>
                  </div>
                {/each}
              {/if}
            </div>
            <div class="rail-foot">
              <button class="foot-row" title={stashes.join("\n")}>
                <span class="mr-icon">⬒</span> Stashed changes
                <span class="mono tiny muted-text pushed-right">{stashes.length}</span>
              </button>
              <button class="foot-row" onclick={() => (centerMode = "releases")}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.6 13.4l-7.2 7.2a2 2 0 01-2.8 0l-7-7a2 2 0 01-.6-1.4V4a1 1 0 011-1h8.2a2 2 0 011.4.6l7 7a2 2 0 010 2.8z" /><circle cx="7.5" cy="7.5" r="1.3" fill="currentColor" /></svg>
                Tags &amp; releases
                <span class="mono tiny muted-text pushed-right">{latestTag?.name ?? "none"}</span>
              </button>
            </div>
          </div>
        {/if}
      </nav>

      <!-- CENTER: history / releases -->
      <section class="center">
        <div class="center-head">
          <div class="seg">
            <button class:on={centerMode === "history"} onclick={() => (centerMode = "history")}>History</button>
            <button class:on={centerMode === "releases"} onclick={() => (centerMode = "releases")}>Releases</button>
            <button class:on={centerMode === "stats"} onclick={openStats}>Stats</button>
          </div>
          {#if centerMode === "history"}
            <div class="seg small">
              <button class:on2={scope === "all"} onclick={() => setScope("all")}>All branches</button>
              <button class:on2={scope === "current"} onclick={() => setScope("current")}>Current</button>
            </div>
          {/if}
          <span class="spacer"></span>
          <span class="mono tiny muted-text">
            {centerMode === "history"
              ? `${commits.length} commits`
              : centerMode === "releases"
                ? `${tags.length} releases`
                : stats
                  ? `${stats.code.toLocaleString()} lines of code`
                  : ""}
          </span>
        </div>

        {#if centerMode === "history"}
          <div class="center-scroll">
            {#each commits as c (c.hash)}
              {@const tip = branchTips.get(c.shortHash) ?? [...branchTips.entries()].find(([h]) => c.hash.startsWith(h))?.[1]}
              {@const isHead = !!status && status.branch.oid !== "" && c.hash.startsWith(status.branch.oid)}
              {@const dotColor = c.parents.length > 1 ? "#2f8f5b" : c.localOnly ? currentColor : "#2f5fe0"}
              <button class="commit-row" class:selected={selectedCommit?.hash === c.hash} onclick={() => selectCommit(c)}>
                <span class="gutter">
                  <span class="gutter-line"></span>
                  <span class="gutter-dot" style="border-color:{dotColor}"></span>
                </span>
                <span class="commit-body">
                  <span class="commit-top">
                    {#if isHead}<span class="head-tag mono" style="background:{dotColor}">HEAD</span>{/if}
                    {#if tip && !isHead}<span class="tip-tag mono" style="background:{tip.color}">⎇ {tip.name}</span>{/if}
                    {#if c.parents.length > 1}<span class="merge-tag mono">⇄ merge</span>{/if}
                    <span class="commit-msg" class:bold={isHead}>{c.subject}</span>
                  </span>
                  <span class="commit-meta mono">
                    <Avatar email={c.email} name={c.author} size={14} />
                    <span>{c.shortHash}</span><span>·</span><span>{c.author}</span><span>·</span><span>{relTime(c.date)}</span>
                    {#if c.localOnly}<span class="warn-text">· local only</span>{/if}
                  </span>
                </span>
              </button>
            {:else}
              <div class="empty-note">No commits yet - make your first one on the right.</div>
            {/each}
            {#if commits.length > 0 && !logExhausted}
              <button class="load-more btn" disabled={loadingMore} onclick={loadMore}>
                {loadingMore ? "Loading…" : "Load more"}
              </button>
            {/if}
          </div>
        {:else if centerMode === "releases"}
          <div class="center-scroll releases">
            <button class="btn-accent create-release" onclick={openReleaseModal}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.2" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>
              Create a release
            </button>
            {#each tags as t, i (t.name)}
              <div class="release-card">
                <div class="release-head">
                  <span class="mono release-version">{t.name}</span>
                  {#if i === 0}<span class="pill latest mono">LATEST</span>{/if}
                  <span class="spacer"></span>
                  <span class="mono tiny muted-text">{relTime(t.date)}</span>
                </div>
                <div class="release-title">{t.subject}</div>
                <div class="release-foot">
                  <span class="mono tag-chip">◈ tag {t.name}</span>
                  <span class="mono tiny muted-text">at {t.hash}</span>
                </div>
              </div>
            {:else}
              <div class="empty-note">No releases yet. A release is a permanent bookmark on one commit - create the first one above.</div>
            {/each}
          </div>
        {:else}
          <div class="center-scroll stats-pane">
            {#if statsLoading && !stats}
              <div class="empty-note">Counting every line…</div>
            {:else if stats}
              <div class="stat-tiles">
                <div class="stat-tile">
                  <span class="stat-num mono">{stats.code.toLocaleString()}</span>
                  <span class="stat-label">lines of code</span>
                </div>
                <div class="stat-tile">
                  <span class="stat-num mono">{stats.files.toLocaleString()}</span>
                  <span class="stat-label">files</span>
                </div>
                <div class="stat-tile">
                  <span class="stat-num mono">{stats.commits.toLocaleString()}</span>
                  <span class="stat-label">commits</span>
                </div>
                <div class="stat-tile">
                  <span class="stat-num mono">
                    {stats.code + stats.comments > 0
                      ? Math.round((stats.comments / (stats.code + stats.comments)) * 100)
                      : 0}%
                  </span>
                  <span class="stat-label">comments</span>
                </div>
                <div class="stat-tile">
                  <span class="stat-num mono">{stats.contributors.length}</span>
                  <span class="stat-label">contributor{stats.contributors.length === 1 ? "" : "s"}</span>
                </div>
                <div class="stat-tile">
                  <span class="stat-num mono">{stats.firstCommitDate ? repoAge(stats.firstCommitDate) : "-"}</span>
                  <span class="stat-label">of history</span>
                </div>
              </div>

              {#if stats.languages.length > 0}
                {@const maxCode = stats.languages[0].code}
                <div class="mono section-label">LANGUAGES</div>
                {#each stats.languages as l (l.name)}
                  <div class="lang-row">
                    <span class="dot" style="background:{langColor(l.name)}"></span>
                    <span class="lang-name">{l.name}</span>
                    <span class="lang-track">
                      <span
                        class="lang-fill"
                        style="width:{Math.max((l.code / maxCode) * 100, 2)}%;background:{langColor(l.name)}"
                      ></span>
                    </span>
                    <span class="mono lang-nums" title="{l.files} files · {l.comments.toLocaleString()} comment lines">
                      {l.code.toLocaleString()}
                    </span>
                  </div>
                {/each}
              {/if}

              {#if stats.contributors.length > 0}
                <div class="mono section-label">CONTRIBUTORS</div>
                {#each stats.contributors as person (person.email + person.name)}
                  <div class="contrib-row">
                    <Avatar email={person.email} name={person.name} size={22} />
                    <span class="contrib-name">{person.name}</span>
                    <span class="contrib-track">
                      <span
                        class="lang-fill"
                        style="width:{Math.max((person.commits / stats.contributors[0].commits) * 100, 2)}%;background:var(--accent)"
                      ></span>
                    </span>
                    <span class="mono lang-nums">{person.commits.toLocaleString()}</span>
                  </div>
                {/each}
              {/if}

              <div class="mono stats-fun">
                {#if stats.code > 0}
                  Printed at 50 lines a page, this codebase is a {Math.max(1, Math.round(stats.code / 50)).toLocaleString()}-page book.
                  {#if stats.comments > 0}
                    There is one comment line for every {Math.max(1, Math.round(stats.code / stats.comments))} lines of code.
                  {/if}
                {/if}
              </div>
              <div class="mono stats-foot">counted by tokei rules · .gitignore respected · recounted each visit</div>
            {:else}
              <div class="empty-note">No stats to show.</div>
            {/if}
          </div>
        {/if}
      </section>

      <!-- RIGHT: working / detail -->
      <div
        class="splitter-v"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize history and changes panes"
        onpointerdown={startColDrag}
      ></div>

      <section class="right" style="width:{rightWidth}px">
        {#if rightView === "working"}
          <div class="right-col">
            <div class="changes-head">
              <button
                class="checkbox"
                class:on={allChecked || someChecked}
                onclick={toggleAll}
                aria-label="select all"
              >
                {#if allChecked}
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 13l4 4L19 7" /></svg>
                {:else if someChecked}
                  <span class="dash"></span>
                {/if}
              </button>
              <span class="mono label">CHANGES</span>
              {#if changeRows.length > 0}
                <span class="count-pill mono">{checkedCount}/{changeRows.length}</span>
              {/if}
              <span class="spacer"></span>
              <button class="ghost-btn" onclick={stashAll} disabled={changeRows.length === 0}>Stash all</button>
            </div>

            <div class="file-list" style="height:{fileListHeight}px">
              {#each changeRows as row (row.path)}
                <div class="file-row" class:dim={!row.checked} class:selected={selectedPath === row.path}>
                  <button class="checkbox small" class:on={row.checked} onclick={() => toggleRow(row)} aria-label="stage">
                    {#if row.checked}
                      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 13l4 4L19 7" /></svg>
                    {/if}
                  </button>
                  <button class="file-main" onclick={() => selectRow(row.path)}>
                    <span class="badge {row.kind}">{badgeOf(row.kind)}</span>
                    <span class="file-path mono" class:strike={!row.checked && false}>{row.path}</span>
                    {#if row.bundleCount}
                      <span class="bundle-pill mono">{row.bundleCount.toLocaleString()} files</span>
                    {/if}
                  </button>
                  <button
                    class="icon-btn"
                    title={row.bundleCount ? "Add this folder to .gitignore" : "Add to .gitignore"}
                    onclick={() => ignoreRow(row.path)}
                  >
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9" /><path d="M5.6 5.6l12.8 12.8" /></svg>
                  </button>
                  {#if !row.bundleCount}
                    <button class="icon-btn danger" title={row.untracked ? "Delete file" : "Discard changes"} onclick={() => discardRow(row)}>×</button>
                  {/if}
                  <span class="mono tiny add-text">+{row.add}</span>
                  <span class="mono tiny del-text">-{row.del}</span>
                </div>
              {:else}
                <div class="empty-note">Working tree clean. Edit some files and they'll show up here.</div>
              {/each}
              {#if ignoredSession.length > 0}
                <div class="ignored-strip">
                  <span class="mono tiny muted-text">⊘ added to .gitignore:</span>
                  {#each ignoredSession as p (p)}
                    <span class="mono tiny ignored-chip">
                      {p}
                      <button class="link-btn" onclick={() => undoIgnore(p)}>undo</button>
                    </span>
                  {/each}
                </div>
              {/if}
            </div>

            <div
              class="splitter-h"
              role="separator"
              aria-orientation="horizontal"
              aria-label="Resize file list and diff"
              onpointerdown={startRowDrag}
            ></div>

            <div class="big-diff">
              {#if selectedRow}
                <div class="diff-head">
                  <span class="mono diff-file">{selectedRow.path}</span>
                  <span class="pill add-pill mono">+{selectedRow.add}</span>
                  <span class="pill del-pill mono">-{selectedRow.del}</span>
                  <span class="spacer"></span>
                  <button class="link-btn mono" onclick={() => openDiffModal(selectedRow!.path, diffText, (full) => gitDiffFile(repo!.path, selectedRow!.path, diffModeFor(selectedRow!.path), full))}>expand ⤢</button>
                </div>
              {/if}
              <div class="diff-scroll">
                <DiffView
                  diff={diffText}
                  emptyMessage={selectedRow?.bundleCount
                    ? `${selectedRow.bundleCount} files in ${selectedRow.path} - too many to preview. Stage them together, or hit the ignore button to keep them out of git.`
                    : changeRows.length === 0
                      ? "Nothing to show"
                      : "Select a file above"}
                />
              </div>
            </div>

            <div class="commit-area">
              {#if onMain}
                <div class="guardrail">
                  <span class="guard-icon">🌱</span>
                  <div class="guard-body">
                    <div class="guard-title">You're committing straight to <span class="mono">{status?.branch.head}</span></div>
                    <div class="guard-sub">Best practice is a branch you can review and undo safely.</div>
                    {#if bannerNaming}
                      <form
                        class="guard-form"
                        onsubmit={(e) => {
                          e.preventDefault();
                          branchFromBanner();
                        }}
                      >
                        <input class="mono" placeholder="feature/my-change" bind:value={bannerBranchName} />
                        <button type="submit" class="btn-warn">Create &amp; switch</button>
                      </form>
                    {:else}
                      <div class="guard-actions">
                        <button class="btn-warn" onclick={() => (bannerNaming = true)}>Create branch first</button>
                        <button class="guard-dismiss" onclick={() => (mainDismissed = true)}>Commit to {status?.branch.head} anyway</button>
                      </div>
                    {/if}
                  </div>
                </div>
              {/if}
              <input
                class="summary mono"
                placeholder="Summary (what &amp; why)"
                bind:value={summary}
                onkeydown={(e) => {
                  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") doCommit();
                }}
              />
              <button class="commit-btn" disabled={!canCommit} onclick={doCommit}>
                Commit {checkedCount} file{checkedCount === 1 ? "" : "s"} →
                <span class="mono" style="color:{currentColor}">{status?.branch.head}</span>
              </button>
            </div>
          </div>
        {:else}
          <!-- commit detail -->
          <div class="right-col">
            <div class="changes-head">
              <span class="mono label">COMMIT</span>
              <span class="mono tiny accent">{selectedCommit?.shortHash}</span>
              <span class="spacer"></span>
              <button class="x-btn big" onclick={backToWorking}>×</button>
            </div>
            <div class="detail-scroll">
              <div class="mono section-label">
                MESSAGE
                {#if selectedEditable}<span class="accent"> · editable</span>{/if}
              </div>
              {#if selectedEditable}
                <textarea
                  class="edit-msg"
                  value={editedMessage ?? details?.message ?? selectedCommit?.subject ?? ""}
                  oninput={(e) => (editedMessage = e.currentTarget.value)}
                ></textarea>
                <div class="edit-actions">
                  <button class="btn-accent small" onclick={saveMessage} disabled={editedMessage === null}>Save message</button>
                  <span class="mono tiny muted-text" title="This commit isn't pushed yet, so it's rewritten in place. No history drama.">what happens?</span>
                </div>
              {:else}
                <div class="msg-box">{details?.message ?? selectedCommit?.subject ?? ""}</div>
                {#if selectedCommit && !selectedCommit.localOnly}
                  <div class="lock-note">🔒 <span title="This commit is on the remote. Changing it would rewrite shared history.">pushed - editing would rewrite shared history</span></div>
                {/if}
              {/if}

              {#if details}
                <div class="author-row">
                  <Avatar email={details.author.email} name={details.author.name} size={28} />
                  <div class="author-col">
                    <span class="author-name">{details.author.name}</span>
                    <span class="mono tiny muted-text">{details.author.email} · {relTime(details.author.date)}</span>
                  </div>
                </div>

                <div class="mono section-label">FILES · {details.files.length}</div>
                {#each details.files as f (f.path)}
                  <div class="detail-file">
                    <span class="badge {f.kind}">{badgeOf(f.kind)}</span>
                    <span class="mono file-path">{#if f.origPath}{f.origPath} → {/if}{f.path}</span>
                    <span class="mono tiny add-text">+{f.additions}</span>
                    <span class="mono tiny del-text">-{f.deletions}</span>
                  </div>
                {/each}
                <button class="link-btn mono view-patch" onclick={() => openDiffModal(`${selectedCommit?.shortHash} ${selectedCommit?.subject}`, commitDiffText, (full) => gitCommitDiff(repo!.path, selectedCommit!.hash, full))}>
                  View full diff ⤢
                </button>
              {/if}
            </div>
            <div class="undo-area">
              <div class="mono section-label">UNDO &amp; EDIT</div>
              <div class="undo-grid">
                <button class="undo-card" disabled={!selectedEditable} onclick={() => undoLast(true)} title={selectedEditable ? "" : "Only the latest unpushed commit can be undone"}>
                  <span class="undo-title">↶ Undo, keep changes</span>
                  <span class="mono tiny muted-text">soft reset · files stay</span>
                </button>
                <button class="undo-card danger" disabled={!selectedEditable} onclick={() => undoLast(false)} title={selectedEditable ? "" : "Only the latest unpushed commit can be undone"}>
                  <span class="undo-title del-text">⌫ Undo &amp; discard</span>
                  <span class="mono tiny muted-text">hard reset · deletes work</span>
                </button>
              </div>
              <div class="undo-secondary">
                <button class="btn" onclick={revertSelected}>Revert (safe)</button>
                <button class="btn" onclick={checkoutHere}>Check out here</button>
              </div>
            </div>
          </div>
        {/if}
      </section>
    </div>
  </main>

  <!-- expanded diff modal -->
  {#if diffModal}
    <button class="overlay" onclick={() => (diffModal = false)} aria-label="close"></button>
    <div class="modal diff-modal">
      <div class="modal-head">
        <span class="mono diff-file">{diffModalTitle}</span>
        <span class="spacer"></span>
        {#if diffModalReload}
          <button
            class="link-btn mono"
            class:accent={diffModalFull}
            onclick={toggleDiffFull}
            disabled={diffModalLoading}
            title="Show every line of the file, not just the changes"
          >
            {diffModalLoading ? "loading…" : diffModalFull ? "Changes only" : "Full file"}
          </button>
        {/if}
        <button class="x-btn big" onclick={() => (diffModal = false)}>×</button>
      </div>
      <div class="modal-diff-scroll">
        <DiffView diff={diffModalText} collapsible />
      </div>
    </div>
  {/if}

  <!-- create release modal -->
  {#if releaseModal}
    <button class="overlay" onclick={() => (releaseModal = false)} aria-label="close"></button>
    <div class="modal release-modal">
      <div class="modal-head">
        <div class="modal-icon">
          <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="M20.6 13.4l-7.2 7.2a2 2 0 01-2.8 0l-7-7a2 2 0 01-.6-1.4V4a1 1 0 011-1h8.2a2 2 0 011.4.6l7 7a2 2 0 010 2.8z" /><circle cx="7.5" cy="7.5" r="1.3" fill="var(--accent)" /></svg>
        </div>
        <span class="modal-title">Create a release</span>
        <span class="spacer"></span>
        <button class="x-btn big" onclick={() => (releaseModal = false)}>×</button>
      </div>
      <div class="modal-body">
        <div class="explainer">
          <span>💡</span>
          <div>A release marks a version of your code you can always come back to. Behind the scenes it creates a <span class="mono accent">tag</span>: a permanent bookmark on <b>one commit</b>, like a labeled save point. Unlike a branch, a tag never moves.</div>
        </div>

        <div class="mono section-label">VERSION</div>
        <div class="version-row">
          <span class="mono v-prefix">v</span>
          <input class="mono version-input" bind:value={version} />
        </div>
        <div class="bump-grid">
          <button class="bump" onclick={() => (version = bumped("patch"))}>
            <span class="bump-name">Patch</span>
            <span class="mono tiny muted-text">v{bumped("patch")}</span>
            <span class="bump-hint">bug fixes</span>
          </button>
          <button class="bump" onclick={() => (version = bumped("minor"))}>
            <span class="bump-name">Minor</span>
            <span class="mono tiny muted-text">v{bumped("minor")}</span>
            <span class="bump-hint">new features</span>
          </button>
          <button class="bump" onclick={() => (version = bumped("major"))}>
            <span class="bump-name">Major</span>
            <span class="mono tiny muted-text">v{bumped("major")}</span>
            <span class="bump-hint">breaking</span>
          </button>
        </div>

        <div class="mono section-label">POINT IT AT</div>
        <div class="target-row">
          <span class="dot" style="background:{currentColor}"></span>
          <span class="target-branch">{status?.branch.head}</span>
          <span class="mono tiny muted-text">latest commit · {commits[0]?.shortHash ?? ""}</span>
        </div>

        <div class="mono section-label">TITLE</div>
        <input class="mono modal-input" placeholder="What would you call this version?" bind:value={relTitle} />

        <div class="notes-head">
          <span class="mono section-label" style="margin:0">RELEASE NOTES</span>
          <span class="spacer"></span>
          <button class="pill-btn mono" onclick={generateNotes}>✨ Generate from commits</button>
        </div>
        <textarea class="modal-notes" placeholder="What changed in this version?" bind:value={relNotes}></textarea>

        <label class="toggle-row">
          <button
            class="checkbox"
            class:on={pushTag}
            onclick={(e) => {
              e.preventDefault();
              pushTag = !pushTag;
            }}
            aria-label="push tag"
          >
            {#if pushTag}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 13l4 4L19 7" /></svg>
            {/if}
          </button>
          <div class="toggle-body">
            <div class="toggle-title">Push the tag to origin</div>
            <div class="mono tiny muted-text">it shows up on GitHub/GitLab under tags (release pages come once accounts connect)</div>
          </div>
        </label>

        <div class="modal-actions">
          <button class="btn" onclick={() => (releaseModal = false)}>Cancel</button>
          <button class="btn-accent" onclick={createRelease} disabled={!version.trim()}>
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.6 13.4l-7.2 7.2a2 2 0 01-2.8 0l-7-7a2 2 0 01-.6-1.4V4a1 1 0 011-1h8.2a2 2 0 011.4.6l7 7a2 2 0 010 2.8z" /></svg>
            Create v{version}
          </button>
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  /* ---------- theme ---------- */
  :global(:root) {
    --desk: #e7e1d7;
    --bg: #f4f0e9;
    --surface: #fffefb;
    --surface2: #faf6ef;
    --ink: #1c1a16;
    --ink2: #4a453d;
    --muted: #8f877b;
    --border: #e7e1d5;
    --border2: #d9d2c4;
    --accent: #2f5fe0;
    --accent-soft: #e8edfd;
    --accent-ink: #1e42a8;
    --clean: #2f8f5b;
    --clean-soft: #e6f3ea;
    --warn: #bd7c22;
    --warn-soft: #fbf0dc;
    --danger: #c0392b;
    --danger-soft: #fbe9e7;
    --add: #2f8f5b;
    --add-soft: #e7f4ea;
    --del: #c0392b;
    --del-soft: #fbe9e7;
  }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --desk: #0e0d0b;
      --bg: #17150f;
      --surface: #201d16;
      --surface2: #28241c;
      --ink: #f3efe6;
      --ink2: #c8c1b3;
      --muted: #8f877a;
      --border: #312c22;
      --border2: #3d382c;
      --accent-soft: #1c2740;
      --accent-ink: #9db6f5;
      --clean-soft: #16281d;
      --warn-soft: #2c2413;
      --danger-soft: #2e1714;
      --add-soft: #16281d;
      --del-soft: #2e1714;
    }
  }
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    background: var(--bg);
    color: var(--ink);
    font-family: "Space Grotesk", system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
  }
  .mono {
    font-family: "JetBrains Mono", ui-monospace, monospace;
  }
  .spacer {
    flex: 1;
  }
  .label {
    font-size: 10.5px;
    letter-spacing: 0.08em;
    color: var(--muted);
    font-weight: 500;
  }
  .tiny {
    font-size: 10px;
  }
  .accent {
    color: var(--accent);
  }
  .accent-text {
    color: var(--accent);
    font-weight: 600;
  }
  .muted-text {
    color: var(--muted);
  }
  .warn-text {
    color: var(--warn);
    font-weight: 600;
  }
  .clean-text {
    color: var(--clean);
  }
  .add-text {
    color: var(--add);
  }
  .del-text {
    color: var(--del);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .pushed-right {
    margin-left: auto;
  }

  /* ---------- buttons ---------- */
  button {
    font: inherit;
    color: inherit;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 38px;
    padding: 0 14px;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 10px;
    font-weight: 600;
    font-size: 12.5px;
    color: var(--ink2);
    cursor: pointer;
  }
  .btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-accent {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 38px;
    padding: 0 14px;
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 10px;
    font-weight: 600;
    font-size: 12.5px;
    cursor: pointer;
  }
  .btn-accent:hover:not(:disabled) {
    filter: brightness(1.07);
  }
  .btn-accent:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .btn-accent.big {
    height: 44px;
    padding: 0 22px;
    font-size: 14px;
    box-shadow: 0 6px 16px -6px var(--accent);
  }
  .btn-accent.small {
    height: 30px;
    padding: 0 11px;
    font-size: 11px;
    border-radius: 8px;
  }
  .btn-warn {
    font-weight: 600;
    font-size: 11px;
    color: #fff;
    background: var(--warn);
    border: none;
    padding: 6px 11px;
    border-radius: 7px;
    cursor: pointer;
  }
  .ghost-btn {
    font-size: 11px;
    color: var(--muted);
    background: none;
    border: none;
    cursor: pointer;
  }
  .ghost-btn:hover:not(:disabled) {
    color: var(--ink);
  }
  .ghost-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .link-btn {
    font-size: 10px;
    color: var(--accent);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .icon-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px;
    color: var(--muted);
    display: flex;
    align-items: center;
    font-size: 13px;
    line-height: 1;
  }
  .icon-btn:hover {
    color: var(--danger);
  }
  .x-btn {
    background: none;
    border: none;
    color: var(--muted);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 2px 4px;
  }
  .x-btn:hover {
    color: var(--danger);
  }
  .x-btn.big {
    font-size: 19px;
  }
  .x-btn.big:hover {
    color: var(--ink);
  }
  .pill {
    font-size: 9.5px;
    padding: 2px 7px;
    border-radius: 20px;
  }
  .pill.clean {
    color: var(--clean);
    background: var(--clean-soft);
  }
  .pill.muted {
    color: var(--muted);
    background: var(--surface2);
  }
  .pill.latest {
    font-size: 9px;
    font-weight: 700;
    color: #fff;
    background: var(--clean);
  }
  .tiny-pill {
    font-size: 8.5px;
    font-weight: 700;
    padding: 1px 6px;
  }
  .add-pill {
    color: var(--add);
    background: var(--add-soft);
  }
  .del-pill {
    color: var(--del);
    background: var(--del-soft);
  }
  .pill-btn {
    font-size: 8.5px;
    font-weight: 700;
    color: var(--accent);
    background: var(--accent-soft);
    border: none;
    padding: 2px 7px;
    border-radius: 20px;
    cursor: pointer;
  }
  .pill-btn.outline {
    color: var(--ink2);
    background: var(--surface);
    border: 1px solid var(--border2);
  }
  .pill-btn.outline:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .checkbox {
    flex: none;
    width: 18px;
    height: 18px;
    border-radius: 5px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    background: var(--surface);
    border: 1px solid var(--border2);
    padding: 0;
  }
  .checkbox.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .checkbox .dash {
    width: 9px;
    height: 2px;
    background: #fff;
    border-radius: 2px;
  }
  .checkbox.small {
    width: 16px;
    height: 16px;
  }

  input,
  textarea {
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 9px;
    color: var(--ink);
    padding: 0.45rem 0.7rem;
    font: inherit;
    font-size: 12.5px;
    outline: none;
  }
  input:focus,
  textarea:focus {
    border-color: var(--accent);
  }
  /* ---------- app frame ---------- */
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 13px;
    padding: 11px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface2);
    flex: none;
  }
  .back {
    font-size: 11px;
    color: var(--muted);
    background: none;
    border: none;
    cursor: pointer;
  }
  .back:hover {
    color: var(--ink);
  }
  .repo-badge {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .repo-icon {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }
  .repo-name {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 1.1;
  }
  .repo-path {
    font-size: 10px;
    color: var(--muted);
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dropdown-anchor {
    position: relative;
  }
  .branch-btn {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 38px;
    padding: 0 12px;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 10px;
    cursor: pointer;
  }
  .branch-btn:hover {
    border-color: var(--muted);
  }
  .branch-name {
    font-weight: 600;
    font-size: 13px;
  }
  .updown {
    font-size: 10.5px;
    color: var(--warn);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 29;
    background: none;
    border: none;
    cursor: default;
  }
  .menu {
    position: absolute;
    left: 0;
    top: 44px;
    width: 250px;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 12px;
    box-shadow: 0 20px 44px -18px rgba(40, 34, 22, 0.5);
    padding: 7px;
    z-index: 30;
  }
  .menu.wide {
    width: 308px;
  }
  .menu.right {
    left: auto;
    right: 0;
  }
  .menu-label {
    font-size: 9px;
    letter-spacing: 0.06em;
    color: var(--muted);
    padding: 6px 9px 7px;
  }
  .menu-row {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 8px 9px;
    background: none;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--ink2);
  }
  .menu-row:hover {
    background: var(--surface2);
  }
  .menu-row-name {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    font-size: 12.5px;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .menu-sep {
    border-top: 1px solid var(--border);
    margin: 5px 4px;
  }
  .menu-card {
    display: flex;
    align-items: flex-start;
    gap: 11px;
    width: 100%;
    padding: 10px 11px;
    background: none;
    border: none;
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
  }
  .menu-card:hover {
    background: var(--accent-soft);
  }
  .menu-card-icon {
    font-size: 15px;
    margin-top: 1px;
  }
  .menu-card-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .menu-card-title {
    font-weight: 600;
    font-size: 12.5px;
    color: var(--ink);
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .menu-card-sub {
    font-size: 10px;
    color: var(--muted);
    line-height: 1.4;
  }
  .mr-icon {
    font-size: 14px;
  }
  .new-branch-form {
    display: flex;
    gap: 6px;
    padding: 6px 8px;
  }
  .new-branch-form input {
    flex: 1;
    min-width: 0;
    font-size: 11px;
    padding: 0.3rem 0.5rem;
  }
  .sync-split {
    display: flex;
  }
  .sync-main {
    border-radius: 10px 0 0 10px;
  }
  .sync-caret {
    width: 30px;
    padding: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.28);
    border-radius: 0 10px 10px 0;
  }

  .banner {
    padding: 8px 16px;
    font-size: 12.5px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--border);
  }
  .error-banner {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .toast-banner {
    background: var(--clean-soft);
    color: var(--clean);
  }
  .banner-close {
    margin-left: auto;
    background: none;
    border: none;
    cursor: pointer;
    color: inherit;
    font-size: 15px;
  }

  /* ---------- panes ---------- */
  .panes {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  /* rail */
  .rail {
    width: 54px;
    flex: none;
    border-right: 1px solid var(--border);
    background: var(--surface2);
    overflow: hidden;
    transition: width 0.2s ease;
  }
  .rail.expanded {
    width: 236px;
  }
  .rail-collapsed {
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
    padding: 16px 0 14px;
    box-sizing: border-box;
  }
  .rail-dots {
    display: flex;
    flex-direction: column;
    gap: 13px;
    align-items: center;
    margin-top: 18px;
    flex: 1;
  }
  .rail-dot {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .rail-vertical {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    font-size: 9px;
    letter-spacing: 0.14em;
    color: var(--muted);
  }
  .rail-expanded {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 236px;
  }
  .rail-head {
    padding: 15px 15px 6px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .plus-btn {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: var(--surface);
    border: 1px solid var(--border2);
    color: var(--ink2);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    line-height: 1;
    padding: 0;
  }
  .plus-btn:hover {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }
  .rail-scroll {
    flex: 1;
    overflow: auto;
    padding: 2px 9px 10px;
  }
  .section-label {
    font-size: 9px;
    letter-spacing: 0.07em;
    color: var(--muted);
    padding: 8px 6px 5px;
    display: block;
  }
  .rail-row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 9px;
    margin-bottom: 2px;
    border-radius: 9px;
    border: 1px solid transparent;
  }
  .rail-row:hover {
    background: var(--surface);
  }
  .rail-row.current {
    border-color: var(--border2);
    background: var(--surface);
    box-shadow: 0 2px 6px -4px rgba(0, 0, 0, 0.28);
  }
  .rail-row-main {
    display: flex;
    align-items: center;
    gap: 9px;
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    text-align: left;
  }
  .rail-row-name {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    font-size: 12.5px;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rail-row.remote .rail-row-name {
    font-weight: 500;
    font-size: 12px;
  }
  .rail-row .x-btn {
    display: none;
  }
  .rail-row:hover .x-btn {
    display: block;
  }
  .rail-foot {
    padding: 10px 15px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .foot-row {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 7px 0;
    background: none;
    border: none;
    color: var(--ink2);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }
  .foot-row:hover {
    color: var(--ink);
  }

  /* center */
  .center {
    flex: 1;
    min-width: 380px;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    min-height: 0;
  }
  .center-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .seg {
    display: flex;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 9px;
    padding: 2px;
  }
  .seg button {
    padding: 5px 13px;
    border: none;
    border-radius: 7px;
    font-weight: 600;
    font-size: 12px;
    cursor: pointer;
    background: transparent;
    color: var(--muted);
  }
  .seg button.on {
    background: var(--accent);
    color: #fff;
  }
  .seg.small button {
    padding: 4px 10px;
    font-size: 11px;
  }
  .seg button.on2 {
    background: var(--surface2);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--border2);
  }
  .center-scroll {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }
  .commit-row {
    display: flex;
    width: 100%;
    background: none;
    border: none;
    border-left: 3px solid transparent;
    cursor: pointer;
    text-align: left;
    padding: 0;
  }
  .commit-row:hover {
    background: var(--surface2);
  }
  .commit-row.selected {
    border-left-color: var(--accent);
    background: var(--surface2);
  }
  .gutter {
    position: relative;
    width: 44px;
    flex: none;
    align-self: stretch;
  }
  .gutter-line {
    position: absolute;
    left: 21px;
    top: 0;
    bottom: 0;
    width: 2.5px;
    background: var(--border2);
  }
  .gutter-dot {
    position: absolute;
    left: 16px;
    top: 50%;
    transform: translateY(-50%);
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--surface);
    border: 3px solid var(--accent);
    box-sizing: border-box;
  }
  .commit-body {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 4px;
    min-height: 58px;
    padding: 8px 16px 8px 4px;
    flex: 1;
    min-width: 0;
  }
  .commit-top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .head-tag {
    font-size: 9px;
    font-weight: 700;
    color: #fff;
    padding: 2px 6px;
    border-radius: 5px;
    letter-spacing: 0.03em;
    flex: none;
  }
  .tip-tag {
    font-size: 9.5px;
    color: #fff;
    padding: 1px 8px;
    border-radius: 20px;
    flex: none;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .merge-tag {
    font-size: 9.5px;
    color: var(--muted);
    background: var(--surface2);
    border: 1px solid var(--border);
    padding: 1px 7px;
    border-radius: 20px;
    flex: none;
  }
  .commit-msg {
    font-size: 13px;
    font-weight: 500;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .commit-msg.bold {
    font-weight: 600;
  }
  .commit-meta {
    font-size: 10.5px;
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .load-more {
    margin: 10px auto 14px;
    display: flex;
  }
  .empty-note {
    color: var(--muted);
    font-size: 12.5px;
    padding: 1.2rem 1rem;
    line-height: 1.5;
  }

  /* releases */
  .center-scroll.releases {
    padding: 16px 18px;
  }

  /* stats */
  .center-scroll.stats-pane {
    padding: 16px 18px;
  }
  .stat-tiles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    margin-bottom: 18px;
  }
  .stat-tile {
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 13px;
    padding: 13px 15px;
  }
  .stat-num {
    font-size: 21px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .stat-label {
    font-size: 11px;
    color: var(--muted);
  }
  .stats-pane .section-label {
    padding: 12px 2px 8px;
  }
  .lang-row,
  .contrib-row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 5px 2px;
  }
  .lang-name,
  .contrib-name {
    width: 110px;
    flex: none;
    font-size: 12.5px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lang-track,
  .contrib-track {
    flex: 1;
    height: 8px;
    background: var(--border);
    border-radius: 20px;
    overflow: hidden;
  }
  .lang-fill {
    display: block;
    height: 100%;
    border-radius: 20px;
  }
  .lang-nums {
    width: 74px;
    flex: none;
    text-align: right;
    font-size: 11px;
    color: var(--ink2);
  }
  .stats-fun {
    margin-top: 18px;
    font-size: 11px;
    line-height: 1.6;
    color: var(--ink2);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 11px;
    padding: 11px 13px;
  }
  .stats-foot {
    margin-top: 10px;
    font-size: 9.5px;
    color: var(--muted);
  }
  .create-release {
    width: 100%;
    height: 44px;
    margin-bottom: 16px;
    box-shadow: 0 4px 12px -6px var(--accent);
  }
  .release-card {
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 13px;
    padding: 15px 16px;
    margin-bottom: 12px;
  }
  .release-head {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .release-version {
    font-size: 15px;
    font-weight: 700;
  }
  .release-title {
    font-size: 13.5px;
    font-weight: 600;
    margin: 8px 0 4px;
  }
  .release-foot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 11px;
  }
  .tag-chip {
    font-size: 10px;
    color: var(--muted);
    background: var(--surface2);
    border: 1px solid var(--border);
    padding: 3px 8px;
    border-radius: 6px;
  }

  /* right pane */
  .right {
    width: 472px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    min-height: 0;
  }
  .splitter-v {
    width: 6px;
    margin: 0 -3px;
    flex: none;
    cursor: col-resize;
    z-index: 5;
    background: none;
  }
  .splitter-v:hover,
  .splitter-v:active {
    background: var(--accent-soft);
  }
  .splitter-h {
    height: 6px;
    margin: -3px 0;
    flex: none;
    cursor: row-resize;
    z-index: 5;
    background: none;
  }
  .splitter-h:hover,
  .splitter-h:active {
    background: var(--accent-soft);
  }
  .right-col {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .changes-head {
    padding: 12px 16px 9px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
  .count-pill {
    font-size: 10px;
    color: #fff;
    background: var(--accent);
    padding: 1px 7px;
    border-radius: 20px;
  }
  .file-list {
    height: 150px;
    overflow: auto;
    padding: 5px 8px;
    border-bottom: 1px solid var(--border);
    flex: none;
    box-sizing: border-box;
  }
  .file-row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 9px;
    border-radius: 8px;
  }
  .file-row:hover {
    background: var(--surface2);
  }
  .file-row.selected {
    background: var(--surface2);
  }
  .file-row.dim {
    opacity: 0.7;
  }
  .file-row .icon-btn {
    visibility: hidden;
  }
  .file-row:hover .icon-btn {
    visibility: visible;
  }
  .file-main {
    display: flex;
    align-items: center;
    gap: 9px;
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    text-align: left;
  }
  .badge {
    flex: none;
    width: 19px;
    height: 19px;
    border-radius: 5px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: "JetBrains Mono", monospace;
    font-weight: 700;
    font-size: 10px;
  }
  .badge.modified,
  .badge.typechange {
    color: var(--warn);
    background: var(--warn-soft);
  }
  .badge.added {
    color: var(--add);
    background: var(--add-soft);
  }
  .badge.deleted,
  .badge.conflict {
    color: var(--del);
    background: var(--del-soft);
  }
  .badge.renamed,
  .badge.copied {
    color: var(--accent);
    background: var(--accent-soft);
  }
  .file-path {
    flex: 1;
    min-width: 0;
    font-size: 11.5px;
    color: var(--ink2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .bundle-pill {
    flex: none;
    font-size: 9.5px;
    color: var(--warn);
    background: var(--warn-soft);
    padding: 1px 7px;
    border-radius: 20px;
  }
  .ignored-strip {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 7px;
    padding: 7px 9px;
  }
  .ignored-chip {
    color: var(--ink2);
    background: var(--surface2);
    border: 1px solid var(--border);
    padding: 1px 7px;
    border-radius: 6px;
    display: inline-flex;
    gap: 5px;
  }

  .big-diff {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--surface2);
    min-height: 0;
  }
  .diff-head {
    padding: 9px 14px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 8px;
    position: sticky;
    top: 0;
    background: var(--surface2);
    flex: none;
  }
  .diff-file {
    font-size: 11.5px;
    color: var(--ink);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .diff-scroll {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .commit-area {
    border-top: 1px solid var(--border);
    padding: 11px 14px;
    flex: none;
  }
  .guardrail {
    display: flex;
    gap: 10px;
    padding: 11px 12px;
    background: var(--warn-soft);
    border: 1px solid #ecd39a;
    border-radius: 11px;
    margin-bottom: 11px;
  }
  .guard-icon {
    font-size: 15px;
    line-height: 1.2;
  }
  .guard-body {
    flex: 1;
  }
  .guard-title {
    font-size: 12px;
    font-weight: 600;
    color: #7a5410;
  }
  .guard-sub {
    font-size: 11px;
    color: #8a6a2a;
    margin-top: 2px;
    line-height: 1.45;
  }
  .guard-actions {
    display: flex;
    gap: 7px;
    margin-top: 9px;
    align-items: center;
  }
  .guard-dismiss {
    font-size: 11px;
    color: #8a6a2a;
    background: none;
    border: none;
    cursor: pointer;
    text-decoration: underline;
  }
  .guard-form {
    display: flex;
    gap: 7px;
    margin-top: 9px;
  }
  .guard-form input {
    flex: 1;
    min-width: 0;
    font-size: 11px;
    padding: 0.3rem 0.5rem;
  }
  .summary {
    width: 100%;
    height: 36px;
    box-sizing: border-box;
    margin-bottom: 8px;
    font-size: 12px;
  }
  .commit-btn {
    width: 100%;
    height: 40px;
    background: var(--ink);
    color: var(--bg);
    border: none;
    border-radius: 10px;
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
  }
  .commit-btn:hover:not(:disabled) {
    filter: brightness(1.15);
  }
  .commit-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }

  /* detail view */
  .detail-scroll {
    padding: 15px 16px;
    overflow: auto;
    flex: 1;
    min-height: 0;
  }
  .edit-msg {
    width: 100%;
    min-height: 64px;
    box-sizing: border-box;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
    font-size: 13px;
    line-height: 1.5;
    resize: vertical;
  }
  .edit-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0 6px;
  }
  .msg-box {
    padding: 11px 12px;
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: 10px;
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
  }
  .lock-note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    font-size: 11px;
    color: var(--warn);
  }
  .author-row {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-top: 14px;
  }
  .author-col {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .author-name {
    font-size: 12.5px;
    font-weight: 600;
  }
  .detail-file {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 7px 9px;
    border-radius: 8px;
    margin-bottom: 3px;
    background: var(--surface2);
  }
  .detail-file .file-path {
    font-size: 11px;
  }
  .view-patch {
    margin-top: 10px;
    font-size: 11px;
  }
  .undo-area {
    border-top: 1px solid var(--border);
    padding: 12px 14px;
    background: var(--surface2);
    flex: none;
  }
  .undo-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .undo-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 9px 11px;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
  }
  .undo-card:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .undo-card.danger:hover:not(:disabled) {
    border-color: var(--danger);
    background: var(--danger-soft);
  }
  .undo-card:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .undo-title {
    font-weight: 600;
    font-size: 11.5px;
    color: var(--ink);
  }
  .undo-secondary {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .undo-secondary .btn {
    flex: 1;
    height: 36px;
    justify-content: center;
    font-weight: 500;
    font-size: 11.5px;
  }

  /* modals */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(28, 26, 22, 0.42);
    backdrop-filter: blur(2px);
    z-index: 50;
    border: none;
    cursor: default;
  }
  .modal {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 16px;
    box-shadow: 0 40px 90px -30px rgba(40, 34, 22, 0.6);
    z-index: 51;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .diff-modal {
    width: 960px;
    max-width: 94vw;
    height: 640px;
    max-height: 90vh;
  }
  .modal-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--surface2);
    flex: none;
  }
  .modal-diff-scroll {
    flex: 1;
    overflow: auto;
    background: var(--surface2);
    min-height: 0;
  }
  .release-modal {
    width: 520px;
    max-width: 94vw;
    max-height: 88vh;
  }
  .modal-icon {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: var(--accent-soft);
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }
  .modal-title {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .modal-body {
    padding: 18px 24px 22px;
    overflow: auto;
    min-height: 0;
  }
  .explainer {
    display: flex;
    gap: 11px;
    padding: 12px 14px;
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: 11px;
    margin-bottom: 20px;
    font-size: 12px;
    line-height: 1.55;
    color: var(--ink2);
  }
  .version-row {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-bottom: 9px;
  }
  .v-prefix {
    font-size: 18px;
    font-weight: 700;
    color: var(--muted);
  }
  .version-input {
    flex: 1;
    height: 44px;
    box-sizing: border-box;
    font-size: 17px;
    font-weight: 700;
  }
  .bump-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 8px;
    margin-bottom: 20px;
  }
  .bump {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 9px 4px;
    background: var(--surface2);
    border: 1px solid var(--border2);
    border-radius: 10px;
    cursor: pointer;
  }
  .bump:hover {
    border-color: var(--accent);
  }
  .bump-name {
    font-weight: 600;
    font-size: 12px;
  }
  .bump-hint {
    font-size: 9.5px;
    color: var(--muted);
    margin-top: 3px;
  }
  .target-row {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 42px;
    padding: 0 13px;
    background: var(--surface2);
    border: 1px solid var(--border2);
    border-radius: 10px;
    margin-bottom: 20px;
  }
  .target-branch {
    font-weight: 600;
    font-size: 12.5px;
  }
  .modal-input {
    width: 100%;
    height: 42px;
    box-sizing: border-box;
    margin-bottom: 16px;
  }
  .notes-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .modal-notes {
    width: 100%;
    min-height: 90px;
    box-sizing: border-box;
    line-height: 1.6;
    resize: vertical;
    margin-bottom: 18px;
    font-size: 12.5px;
  }
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 11px 13px;
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: 10px;
    cursor: pointer;
    margin-bottom: 20px;
  }
  .toggle-title {
    font-size: 12.5px;
    font-weight: 600;
  }
  .toggle-body {
    flex: 1;
  }
  .modal-actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }
  .release-modal .section-label {
    font-size: 11px;
    letter-spacing: 0.05em;
    padding: 0;
    margin-bottom: 8px;
  }
</style>
