<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import {
    repoSummary,
    cloneRepo,
    initRepo,
    openInEditor,
    openInTerminal,
    scanRepos,
    githubAccount,
    githubRepos,
    githubConnect,
    githubDisconnect,
    errorMessage,
    type RepoSummary,
    type FoundRepo,
    type GithubUser,
    type GithubRepo,
  } from "$lib/git";

  let { onopen }: { onopen: (path: string) => void } = $props();

  const RECENT_KEY = "trident.recentRepos";
  const DIR_KEY = "trident.projectsDir";

  interface RecentRepo {
    path: string;
    name: string;
  }

  let projects = $state<RecentRepo[]>(loadRecent());
  let summaries = $state<Record<string, RepoSummary | "missing" | undefined>>({});
  let filter = $state<"All" | "Local" | "GitHub" | "GitLab">("All");
  let query = $state("");
  let addOpen = $state(false);
  let error = $state<string | null>(null);
  let searchEl = $state<HTMLInputElement | null>(null);

  let cloneModal = $state(false);
  let cloneUrl = $state("");
  let newModal = $state(false);
  let newName = $state("");
  let projectsDir = $state<string>(localStorage.getItem(DIR_KEY) ?? "");
  let working = $state(false);

  // GitHub
  let ghUser = $state<GithubUser | null>(null);
  let ghChecked = $state(false);
  let ghRepos = $state<GithubRepo[]>([]);
  let ghError = $state<string | null>(null);
  let accountModal = $state(false);
  let tokenInput = $state("");

  // repo scanner
  let scanModal = $state(false);
  let scanning = $state(false);
  let scanResults = $state<FoundRepo[]>([]);
  let scanSelected = $state<Record<string, boolean>>({});

  $effect(() => {
    githubAccount()
      .then(async (u) => {
        ghUser = u;
        ghChecked = true;
        if (u) ghRepos = await githubRepos();
      })
      .catch((e) => {
        ghChecked = true;
        ghError = errorMessage(e);
      });
  });

  /// "github.com/user/repo" from any https/ssh remote URL variant.
  function normalizeGitUrl(url: string): string {
    return url
      .trim()
      .toLowerCase()
      .replace(/^https?:\/\//, "")
      .replace(/^ssh:\/\//, "")
      .replace(/^git@/, "")
      .replace(":", "/")
      .replace(/\.git$/, "")
      .replace(/\/+$/, "");
  }

  /// Local project path for a GitHub repo, when a clone already exists.
  let localPathByUrl = $derived.by(() => {
    const map = new Map<string, string>();
    for (const p of projects) {
      const s = summaries[p.path];
      if (s && s !== "missing" && s.originUrl) {
        map.set(normalizeGitUrl(s.originUrl), p.path);
      }
    }
    return map;
  });
  const clonePathOf = (r: GithubRepo) => localPathByUrl.get(normalizeGitUrl(r.cloneUrl)) ?? null;

  const LANG_COLORS: Record<string, string> = {
    Rust: "#b7410e",
    TypeScript: "#3178c6",
    JavaScript: "#f1c40f",
    Svelte: "#ff3e00",
    Vue: "#42b883",
    Python: "#3572a5",
    Ruby: "#cc342d",
    Go: "#00add8",
    Java: "#b07219",
    Kotlin: "#a97bff",
    Swift: "#f05138",
    C: "#555555",
    "C++": "#f34b7d",
    "C#": "#178600",
    PHP: "#4f5d95",
    Elixir: "#6e4a7e",
    Zig: "#ec915c",
    Lua: "#000080",
    CSS: "#563d7c",
    HTML: "#e34c26",
    Shell: "#89e051",
    SQL: "#336791",
    Astro: "#ff5d01",
    Jupyter: "#f37726",
    Docker: "#2496ed",
  };
  const langColor = (name: string) => LANG_COLORS[name] ?? "#8f877b";

  function loadRecent(): RecentRepo[] {
    try {
      return JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    } catch {
      return [];
    }
  }

  function saveRecent() {
    localStorage.setItem(RECENT_KEY, JSON.stringify(projects));
  }

  $effect(() => {
    for (const p of projects) {
      if (summaries[p.path] === undefined) {
        repoSummary(p.path)
          .then((s) => (summaries = { ...summaries, [p.path]: s }))
          .catch(() => (summaries = { ...summaries, [p.path]: "missing" }));
      }
    }
  });

  let visible = $derived.by(() => {
    if (filter === "GitHub" || filter === "GitLab") return [];
    const q = query.trim().toLowerCase();
    return projects.filter((p) => {
      if (!q) return true;
      const s = summaries[p.path];
      const langs = s && s !== "missing" ? s.langs.map((l) => l.name.toLowerCase()) : [];
      return p.name.toLowerCase().includes(q) || langs.some((l) => l.includes(q));
    });
  });

  /// GitHub cards: everything on the GitHub tab; only not-yet-cloned repos
  /// on All (their clones are already there as local cards).
  let ghVisible = $derived.by(() => {
    if (filter === "Local" || filter === "GitLab" || !ghUser) return [];
    const q = query.trim().toLowerCase();
    return ghRepos.filter((r) => {
      if (filter === "All" && clonePathOf(r)) return false;
      if (!q) return true;
      return (
        r.name.toLowerCase().includes(q) || (r.language ?? "").toLowerCase().includes(q)
      );
    });
  });

  async function connectGithub() {
    if (!tokenInput.trim()) return;
    working = true;
    ghError = null;
    try {
      ghUser = await githubConnect(tokenInput);
      tokenInput = "";
      accountModal = false;
      ghRepos = await githubRepos();
    } catch (e) {
      ghError = errorMessage(e);
    }
    working = false;
  }

  async function disconnectGithub() {
    await githubDisconnect();
    // env/gh tokens are outside the keychain; re-resolve to see what's left.
    ghUser = await githubAccount().catch(() => null);
    if (!ghUser) ghRepos = [];
    accountModal = false;
  }

  function cloneFromGithub(r: GithubRepo) {
    cloneUrl = r.cloneUrl;
    error = null;
    cloneModal = true;
  }

  async function openScan() {
    addOpen = false;
    scanModal = true;
    scanning = true;
    scanResults = [];
    try {
      const found = await scanRepos();
      const known = new Set(projects.map((p) => p.path));
      scanResults = found;
      const sel: Record<string, boolean> = {};
      for (const f of found) sel[f.path] = !known.has(f.path);
      scanSelected = sel;
    } catch (e) {
      error = errorMessage(e);
    }
    scanning = false;
  }

  let scanNew = $derived(scanResults.filter((f) => !projects.some((p) => p.path === f.path)));
  let scanPickedCount = $derived(scanNew.filter((f) => scanSelected[f.path]).length);

  function addScanned() {
    const picked = scanNew.filter((f) => scanSelected[f.path]);
    projects = [...picked.map((f) => ({ path: f.path, name: f.name })), ...projects];
    saveRecent();
    scanModal = false;
  }

  function relTime(iso: string): string {
    const s = (Date.now() - new Date(iso).getTime()) / 1000;
    if (s < 90) return "just now";
    if (s < 3600) return `${Math.round(s / 60)}m ago`;
    if (s < 86400) return `${Math.round(s / 3600)}h ago`;
    if (s < 604800) return `${Math.round(s / 86400)}d ago`;
    return new Date(iso).toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }

  function removeProject(path: string) {
    projects = projects.filter((p) => p.path !== path);
    saveRecent();
  }

  async function addExisting() {
    addOpen = false;
    const picked = await openDialog({ directory: true, title: "Add existing repository" });
    if (typeof picked === "string") onopen(picked);
  }

  async function browseDir() {
    const picked = await openDialog({ directory: true, title: "Where should projects live?" });
    if (typeof picked === "string") {
      projectsDir = picked;
      localStorage.setItem(DIR_KEY, picked);
    }
  }

  function cloneNameFromUrl(url: string): string {
    const last = url.replace(/\/+$/, "").split("/").pop() ?? "";
    return last.replace(/\.git$/, "").replace(/^.*:/, "");
  }

  async function doClone() {
    const name = cloneNameFromUrl(cloneUrl);
    if (!cloneUrl.trim() || !projectsDir || !name || working) return;
    working = true;
    error = null;
    try {
      const info = await cloneRepo(cloneUrl.trim(), `${projectsDir}/${name}`);
      cloneModal = false;
      cloneUrl = "";
      onopen(info.path);
    } catch (e) {
      error = errorMessage(e);
    }
    working = false;
  }

  async function doCreate() {
    const name = newName.trim();
    if (!name || !projectsDir || working) return;
    working = true;
    error = null;
    try {
      const info = await initRepo(`${projectsDir}/${name}`);
      newModal = false;
      newName = "";
      onopen(info.path);
    } catch (e) {
      error = errorMessage(e);
    }
    working = false;
  }

  async function launch(fn: (path: string) => Promise<void>, path: string) {
    try {
      await fn(path);
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      searchEl?.focus();
    }
  }}
/>

<main class="hub">
  <!-- top strip -->
  <div class="chrome mono">
    <span>trident — projects</span>
    <span class="spacer"></span>
    <span>⌘K to search</span>
  </div>

  <!-- header -->
  <div class="head">
    <div class="brand">
      <div class="brand-icon">
        <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round"><circle cx="6" cy="6" r="2.4" /><circle cx="6" cy="18" r="2.4" /><circle cx="18" cy="9" r="2.4" /><path d="M6 8.4v7.2M8.2 6.6c6 0 7.6 1 7.6 4.2" /></svg>
      </div>
      <div>
        <div class="brand-title">Projects</div>
        <div class="mono brand-sub">{projects.length} project{projects.length === 1 ? "" : "s"} · {projects.length} on this machine</div>
      </div>
    </div>

    <div class="seg">
      {#each ["All", "Local", "GitHub", "GitLab"] as f (f)}
        <button class:on={filter === f} onclick={() => (filter = f as typeof filter)}>{f}</button>
      {/each}
    </div>

    <span class="spacer"></span>

    <div class="search">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="var(--muted)" stroke-width="2.2" stroke-linecap="round"><circle cx="11" cy="11" r="7" /><path d="M21 21l-4-4" /></svg>
      <input class="mono" placeholder="Search name or language…" bind:value={query} bind:this={searchEl} />
    </div>

    <div class="add-anchor">
      <button class="add-btn" onclick={() => (addOpen = !addOpen)}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>
        Add
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.6" stroke-linecap="round" style="opacity:.8"><path d="M6 9l6 6 6-6" /></svg>
      </button>
      {#if addOpen}
        <button class="scrim" onclick={() => (addOpen = false)} aria-label="close"></button>
        <div class="add-menu">
          <button class="add-item" onclick={() => { addOpen = false; cloneModal = true; error = null; }}>
            <span class="add-icon">🔗</span>
            <span class="add-body">
              <span class="add-label">Clone from URL…</span>
              <span class="mono add-hint">paste any git remote</span>
            </span>
          </button>
          <button class="add-item" onclick={addExisting}>
            <span class="add-icon">📁</span>
            <span class="add-body">
              <span class="add-label">Add existing folder…</span>
              <span class="mono add-hint">a repo already on disk</span>
            </span>
          </button>
          <button class="add-item" onclick={openScan}>
            <span class="add-icon">🔍</span>
            <span class="add-body">
              <span class="add-label">Scan for existing repos…</span>
              <span class="mono add-hint">searches your usual project folders</span>
            </span>
          </button>
          <button class="add-item last" onclick={() => { addOpen = false; newModal = true; error = null; }}>
            <span class="add-icon">＋</span>
            <span class="add-body">
              <span class="add-label">Create new repo…</span>
              <span class="mono add-hint">start from scratch</span>
            </span>
          </button>
        </div>
      {/if}
    </div>
  </div>

  <!-- accounts strip -->
  <div class="accounts">
    <span class="acct github">
      <svg width="13" height="13" viewBox="0 0 16 16" fill="var(--github)"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z" /></svg>
      {#if ghUser}
        <img class="acct-avatar" src={ghUser.avatarUrl} alt={ghUser.login} />
        github.com/{ghUser.login}
      {:else if !ghChecked}
        GitHub · checking…
      {:else}
        GitHub · not connected
      {/if}
    </span>
    <span class="acct gitlab">
      <svg width="13" height="13" viewBox="0 0 16 16" fill="var(--gitlab)"><path d="M8 15.5l2.94-9.05H5.06L8 15.5z" opacity=".9" /><path d="M8 15.5L5.06 6.45H1.34L8 15.5zM8 15.5l2.94-9.05h3.72L8 15.5z" /><path d="M1.34 6.45L.45 9.2c-.08.25 0 .52.22.67L8 15.5 1.34 6.45zM14.66 6.45l.89 2.75c.08.25 0 .52-.22.67L8 15.5l6.66-9.05z" /></svg>
      GitLab · not connected
    </span>
    <span class="mono acct-note">
      {#if ghUser}
        {ghRepos.length} GitHub repos found · {ghRepos.filter((r) => clonePathOf(r)).length} cloned locally
      {:else}
        connect GitHub to see your repos here · GitLab support is coming
      {/if}
    </span>
    <span class="spacer"></span>
    <button class="manage-btn" onclick={() => { accountModal = true; ghError = null; }}>Manage accounts</button>
  </div>

  {#if error && !cloneModal && !newModal}
    <div class="hub-error">{error}<button class="x-close" onclick={() => (error = null)}>×</button></div>
  {/if}

  <!-- grid -->
  <div class="grid-wrap">
    {#if filter === "GitLab"}
      <div class="no-results">
        <div class="nr-title">No GitLab account connected</div>
        <div class="mono nr-sub">GitLab support is coming</div>
      </div>
    {:else if filter === "GitHub" && !ghUser}
      <div class="no-results">
        <div class="nr-title">No GitHub account connected</div>
        <div class="mono nr-sub">connect via Manage accounts to see your repos</div>
        <button class="add-btn nr-connect" onclick={() => { accountModal = true; ghError = null; }}>Connect GitHub</button>
      </div>
    {:else if visible.length === 0 && ghVisible.length === 0}
      <div class="no-results">
        <div class="nr-title">{projects.length === 0 ? "No projects yet" : "No projects match"}</div>
        <div class="mono nr-sub">
          {projects.length === 0 ? "Use Add to clone, create, scan, or pick a folder" : "Try a different filter or search term"}
        </div>
      </div>
    {:else}
      <div class="grid">
        {#each visible as p (p.path)}
          {@const s = summaries[p.path]}
          <div class="card">
            <button class="card-remove" title="Remove from this list (keeps the folder)" onclick={() => removeProject(p.path)}>×</button>
            <div class="card-top">
              <span class="pill-badge mono">● local</span>
              {#if s === "missing"}
                <span class="pill-status mono missing">missing</span>
              {:else if s === undefined}
                <span class="pill-status mono neutral">…</span>
              {:else if s.changes > 0}
                <span class="pill-status mono changes">{s.changes} ●</span>
              {:else if s.ahead > 0}
                <span class="pill-status mono unpushed">{s.ahead} unpushed</span>
              {:else}
                <span class="pill-status mono clean">✓ clean</span>
              {/if}
            </div>
            <div class="card-name">{p.name}</div>
            <div class="mono card-meta">
              {#if s === "missing"}
                folder moved or deleted
              {:else if s}
                ⎇ {s.branch}{#if s.lastCommitDate}&nbsp;· updated {relTime(s.lastCommitDate)}{/if}
              {:else}
                {p.path}
              {/if}
            </div>

            <div class="lang-bar">
              {#if s && s !== "missing" && s.langs.length > 0}
                {#each s.langs as l (l.name)}
                  <span style="flex:{l.pct};background:{langColor(l.name)}"></span>
                {/each}
              {/if}
            </div>
            <div class="mono lang-legend">
              {#if s && s !== "missing" && s.langs.length > 0}
                {s.langs.map((l) => `${l.name} ${l.pct}%`).join(" · ")}
              {:else}
                &nbsp;
              {/if}
            </div>

            <div class="tags">
              {#if s && s !== "missing"}
                {#each s.langs as l (l.name)}
                  <span class="tag">
                    <i style="background:{langColor(l.name)}"></i>
                    {l.name}
                  </span>
                {/each}
              {/if}
            </div>

            <div class="actions">
              <button class="act" onclick={() => launch(openInEditor, p.path)} disabled={s === "missing"}>◫ VS Code</button>
              <button class="act mono" onclick={() => launch(openInTerminal, p.path)} disabled={s === "missing"}>&gt;_</button>
              <button class="act primary" onclick={() => onopen(p.path)} disabled={s === "missing"}>Open ▸</button>
            </div>
          </div>
        {/each}
        {#each ghVisible as r (r.fullName)}
          {@const localPath = clonePathOf(r)}
          <div class="card">
            <div class="card-top">
              <span class="pill-badge gh mono">☁ GitHub{r.private ? " · private" : ""}{r.fork ? " · fork" : ""}</span>
              {#if localPath}
                <span class="pill-status mono clean">✓ cloned</span>
              {:else}
                <span class="pill-status mono missing">not cloned</span>
              {/if}
            </div>
            <div class="card-name">{r.name}</div>
            <div class="mono card-meta">
              ★ {r.stars}{#if r.pushedAt}&nbsp;· updated {relTime(r.pushedAt)}{/if}
            </div>
            <div class="lang-bar">
              {#if r.language}<span style="flex:1;background:{langColor(r.language)}"></span>{/if}
            </div>
            <div class="mono lang-legend">{r.language ?? ""}&nbsp;</div>
            <div class="tags">
              {#if r.language}
                <span class="tag"><i style="background:{langColor(r.language)}"></i>{r.language}</span>
              {/if}
              {#if r.description}
                <span class="tag desc" title={r.description}>{r.description}</span>
              {/if}
            </div>
            <div class="actions">
              {#if localPath}
                <button class="act primary" onclick={() => onopen(localPath)}>Open ▸</button>
              {:else}
                <button class="act clone" onclick={() => cloneFromGithub(r)}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12m0 0l4-4m-4 4l-4-4M4 17v2a2 2 0 002 2h12a2 2 0 002-2v-2" /></svg>
                  Clone locally
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</main>

<!-- manage accounts modal -->
{#if accountModal}
  <button class="overlay" onclick={() => (accountModal = false)} aria-label="close"></button>
  <div class="modal">
    <div class="modal-top">
      <div class="modal-title">Accounts</div>
      <div class="mono modal-sub">connect GitHub to browse and clone your repos</div>
    </div>
    <div class="modal-body">
      {#if ghUser}
        <div class="connected-row">
          <img class="acct-avatar big" src={ghUser.avatarUrl} alt={ghUser.login} />
          <div class="connected-col">
            <span class="connected-name">{ghUser.name ?? ghUser.login}</span>
            <span class="mono connected-sub">github.com/{ghUser.login} · via {ghUser.tokenSource === "gh" ? "gh CLI" : ghUser.tokenSource === "env" ? "GITHUB_TOKEN" : "saved token"}</span>
          </div>
        </div>
        {#if ghUser.tokenSource === "keychain"}
          <button class="cancel full" onclick={disconnectGithub}>Disconnect</button>
        {:else}
          <div class="mono field-hint">this sign-in comes from outside Trident ({ghUser.tokenSource === "gh" ? "the gh CLI" : "the GITHUB_TOKEN variable"}), manage it there</div>
        {/if}
      {:else}
        <div class="field-label">Personal access token</div>
        <input class="mono field" type="password" placeholder="ghp_… or github_pat_…" bind:value={tokenInput} />
        <div class="mono field-hint">
          create one at github.com/settings/tokens with repo scope · stored in your OS keychain ·
          already using the gh CLI? then you're connected automatically
        </div>
        {#if ghError}<div class="hub-error inline">{ghError}</div>{/if}
        <div class="modal-actions">
          <button class="cancel" onclick={() => (accountModal = false)}>Cancel</button>
          <button class="cta" disabled={!tokenInput.trim() || working} onclick={connectGithub}>
            {working ? "Checking…" : "Connect GitHub"}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- scan modal -->
{#if scanModal}
  <button class="overlay" onclick={() => (scanModal = false)} aria-label="close"></button>
  <div class="modal">
    <div class="modal-top">
      <div class="modal-title">Scan for repositories</div>
      <div class="mono modal-sub">looks a few levels deep in your usual project folders</div>
    </div>
    <div class="modal-body">
      {#if scanning}
        <div class="mono scan-note">Scanning…</div>
      {:else if scanResults.length === 0}
        <div class="mono scan-note">No repositories found in the common folders.</div>
      {:else}
        <div class="scan-list">
          {#each scanResults as f (f.path)}
            {@const known = projects.some((p) => p.path === f.path)}
            <label class="scan-row" class:known>
              <input type="checkbox" disabled={known} bind:checked={scanSelected[f.path]} />
              <span class="scan-name">{f.name}</span>
              <span class="mono scan-path" title={f.path}>{f.path}</span>
              {#if known}<span class="mono scan-known">already added</span>{/if}
            </label>
          {/each}
        </div>
        <div class="mono field-hint">
          {scanResults.length} found · {scanResults.length - scanNew.length} already on the hub
        </div>
      {/if}
      <div class="modal-actions">
        <button class="cancel" onclick={() => (scanModal = false)}>Cancel</button>
        <button class="cta" disabled={scanning || scanPickedCount === 0} onclick={addScanned}>
          Add {scanPickedCount} repo{scanPickedCount === 1 ? "" : "s"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- clone modal -->
{#if cloneModal}
  <button class="overlay" onclick={() => (cloneModal = false)} aria-label="close"></button>
  <div class="modal">
    <div class="modal-top">
      <div class="modal-title">Clone a repository</div>
      <div class="mono modal-sub">paste any git remote URL: https or ssh</div>
    </div>
    <div class="modal-body">
      <div class="field-label">URL</div>
      <input class="mono field" placeholder="https://github.com/you/repo.git" bind:value={cloneUrl} />
      <div class="field-label">Location</div>
      <div class="loc-row">
        <div class="mono loc-path">{projectsDir ? `${projectsDir}/${cloneNameFromUrl(cloneUrl) || "…"}` : "pick a projects folder"}</div>
        <button class="browse" onclick={browseDir}>Browse…</button>
      </div>
      <div class="mono field-hint">default folder · change once, remembered after</div>
      {#if error}<div class="hub-error inline">{error}</div>{/if}
      <div class="modal-actions">
        <button class="cancel" onclick={() => (cloneModal = false)}>Cancel</button>
        <button class="cta" disabled={!cloneUrl.trim() || !projectsDir || working} onclick={doClone}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12m0 0l4-4m-4 4l-4-4M4 17v2a2 2 0 002 2h12a2 2 0 002-2v-2" /></svg>
          {working ? "Cloning…" : "Clone here"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- new repo modal -->
{#if newModal}
  <button class="overlay" onclick={() => (newModal = false)} aria-label="close"></button>
  <div class="modal">
    <div class="modal-top">
      <div class="modal-title">Create a new repository</div>
      <div class="mono modal-sub">an empty repo on branch main, ready for a first commit</div>
    </div>
    <div class="modal-body">
      <div class="field-label">Name</div>
      <input class="mono field" placeholder="my-new-project" bind:value={newName} />
      <div class="field-label">Location</div>
      <div class="loc-row">
        <div class="mono loc-path">{projectsDir ? `${projectsDir}/${newName.trim() || "…"}` : "pick a projects folder"}</div>
        <button class="browse" onclick={browseDir}>Browse…</button>
      </div>
      <div class="mono field-hint">default folder · change once, remembered after</div>
      {#if error}<div class="hub-error inline">{error}</div>{/if}
      <div class="modal-actions">
        <button class="cancel" onclick={() => (newModal = false)}>Cancel</button>
        <button class="cta" disabled={!newName.trim() || !projectsDir || working} onclick={doCreate}>
          {working ? "Creating…" : "Create repo"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .mono {
    font-family: "JetBrains Mono", ui-monospace, monospace;
  }
  .spacer {
    flex: 1;
  }
  .hub {
    min-height: 100vh;
    background: radial-gradient(120% 90% at 50% 0%, var(--surface2) 0%, var(--bg) 70%);
    display: flex;
    flex-direction: column;
  }
  .chrome {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 11px 20px;
    background: var(--surface2);
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px 24px 18px;
    flex-wrap: wrap;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .brand-icon {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    background: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 12px -4px var(--accent);
  }
  .brand-title {
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 1;
  }
  .brand-sub {
    font-size: 11px;
    color: var(--muted);
    margin-top: 3px;
  }
  .seg {
    display: flex;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 10px;
    padding: 3px;
    margin-left: 8px;
  }
  .seg button {
    padding: 6px 14px;
    border: none;
    border-radius: 7px;
    font: 600 12.5px "Space Grotesk", sans-serif;
    cursor: pointer;
    transition: 0.12s;
    background: transparent;
    color: var(--muted);
  }
  .seg button.on {
    background: var(--accent);
    color: #fff;
    box-shadow: 0 3px 8px -4px var(--accent);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 10px;
    padding: 0 12px;
    height: 38px;
    width: 220px;
  }
  .search input {
    border: none;
    outline: none;
    background: none;
    font-size: 12px;
    color: var(--ink);
    width: 100%;
    padding: 0;
  }
  .add-anchor {
    position: relative;
  }
  .add-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 38px;
    padding: 0 15px;
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 10px;
    font: 600 13px "Space Grotesk", sans-serif;
    cursor: pointer;
    box-shadow: 0 4px 12px -5px var(--accent);
  }
  .add-btn:hover {
    filter: brightness(1.06);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 29;
    background: none;
    border: none;
    cursor: default;
  }
  .add-menu {
    position: absolute;
    right: 0;
    top: 46px;
    width: 250px;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 12px;
    box-shadow: 0 20px 44px -18px rgba(40, 34, 22, 0.5);
    padding: 6px;
    z-index: 30;
  }
  .add-item {
    display: flex;
    align-items: center;
    gap: 11px;
    width: 100%;
    padding: 9px 10px;
    border: none;
    border-radius: 9px;
    cursor: pointer;
    background: none;
    text-align: left;
  }
  .add-item:hover {
    background: var(--surface2);
  }
  .add-item.last {
    border-top: 1px solid var(--border);
    border-radius: 0 0 9px 9px;
    margin-top: 4px;
    padding-top: 12px;
  }
  .add-icon {
    font-size: 15px;
    width: 20px;
    text-align: center;
  }
  .add-body {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .add-label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--ink);
  }
  .add-hint {
    font-size: 10px;
    color: var(--muted);
  }

  .accounts {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 24px;
    background: var(--surface2);
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  .acct {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--ink2);
    padding: 4px 10px;
    border-radius: 20px;
  }
  .acct.github {
    background: var(--github-soft, #f2edfb);
    border: 1px solid #e3d8f5;
  }
  .acct.gitlab {
    background: var(--gitlab-soft, #fcefe8);
    border: 1px solid #f3d8ca;
  }
  .acct-note {
    font-size: 11px;
    color: var(--muted);
    margin-left: 2px;
  }
  .acct-avatar {
    width: 16px;
    height: 16px;
    border-radius: 50%;
  }
  .acct-avatar.big {
    width: 38px;
    height: 38px;
  }
  .manage-btn {
    font-size: 11px;
    font-weight: 500;
    color: var(--muted);
    background: none;
    border: none;
    cursor: pointer;
  }
  .manage-btn:hover {
    color: var(--ink);
  }
  .pill-badge.gh {
    background: var(--github-soft, #f2edfb);
    border-color: #e3d8f5;
    color: var(--github);
  }
  .tag.desc {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
  }
  .act.clone {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
    box-shadow: 0 4px 12px -6px var(--accent);
  }
  .act.clone:hover:not(:disabled) {
    filter: brightness(1.07);
    background: var(--accent);
  }
  .nr-connect {
    margin: 18px auto 0;
  }
  .connected-row {
    display: flex;
    align-items: center;
    gap: 11px;
    margin-bottom: 16px;
  }
  .connected-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .connected-name {
    font-size: 14px;
    font-weight: 600;
  }
  .connected-sub {
    font-size: 11px;
    color: var(--muted);
  }
  .cancel.full {
    width: 100%;
  }
  .scan-note {
    font-size: 12px;
    color: var(--muted);
    padding: 8px 0 16px;
  }
  .scan-list {
    max-height: 300px;
    overflow: auto;
    margin-bottom: 8px;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 4px;
  }
  .scan-row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 7px 9px;
    border-radius: 8px;
    cursor: pointer;
  }
  .scan-row:hover {
    background: var(--surface2);
  }
  .scan-row.known {
    opacity: 0.55;
    cursor: default;
  }
  .scan-name {
    font-size: 12.5px;
    font-weight: 600;
    flex: none;
  }
  .scan-path {
    font-size: 10px;
    color: var(--muted);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .scan-known {
    font-size: 9.5px;
    color: var(--clean);
    flex: none;
  }

  .hub-error {
    margin: 12px 24px 0;
    padding: 9px 13px;
    background: var(--danger-soft);
    color: var(--danger);
    border-radius: 9px;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .hub-error.inline {
    margin: 0 0 12px;
  }
  .x-close {
    margin-left: auto;
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 14px;
  }

  .grid-wrap {
    padding: 22px 24px 28px;
    flex: 1;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 16px 16px 15px;
    transition:
      transform 0.16s ease,
      box-shadow 0.16s ease,
      border-color 0.16s ease;
  }
  .card:hover {
    border-color: var(--border2);
    box-shadow: 0 12px 30px -16px rgba(40, 34, 22, 0.42);
    transform: translateY(-2px);
  }
  .card-remove {
    position: absolute;
    top: 8px;
    right: 10px;
    display: none;
    background: none;
    border: none;
    color: var(--muted);
    font-size: 15px;
    cursor: pointer;
    z-index: 2;
  }
  .card:hover .card-remove {
    display: block;
  }
  .card-remove:hover {
    color: var(--danger);
  }
  .card-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
    padding-right: 14px;
  }
  .pill-badge,
  .pill-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 10.5px;
    font-weight: 600;
    padding: 3px 9px;
    border-radius: 20px;
    white-space: nowrap;
  }
  .pill-badge {
    background: var(--surface2);
    border: 1px solid var(--border2);
    color: var(--ink2);
  }
  .pill-status.clean {
    background: var(--clean-soft);
    color: var(--clean);
  }
  .pill-status.changes {
    background: var(--accent-soft);
    color: var(--accent-ink, var(--accent));
  }
  .pill-status.unpushed {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .pill-status.missing {
    border: 1px solid var(--border2);
    color: var(--muted);
  }
  .pill-status.neutral {
    color: var(--muted);
  }
  .card-name {
    font-size: 16.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
    margin: 11px 0 3px;
    color: var(--ink);
  }
  .card-meta {
    font-size: 11px;
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lang-bar {
    display: flex;
    height: 6px;
    border-radius: 20px;
    overflow: hidden;
    margin: 14px 0 7px;
    background: var(--border);
  }
  .lang-bar span {
    display: block;
  }
  .lang-legend {
    font-size: 10px;
    color: var(--muted);
    margin-bottom: 13px;
    min-height: 12px;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-bottom: 15px;
    min-height: 22px;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10.5px;
    font-weight: 500;
    color: var(--ink2);
    background: var(--surface2);
    border: 1px solid var(--border);
    padding: 2px 8px;
    border-radius: 6px;
  }
  .tag i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    display: block;
  }
  .actions {
    display: flex;
    gap: 7px;
    margin-top: auto;
  }
  .act {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 34px;
    background: var(--surface2);
    border: 1px solid var(--border2);
    border-radius: 8px;
    font: 500 11.5px "Space Grotesk", sans-serif;
    color: var(--ink2);
    cursor: pointer;
  }
  .act:hover:not(:disabled) {
    background: var(--bg);
  }
  .act.mono {
    font-family: "JetBrains Mono", monospace;
  }
  .act.primary {
    flex: 1.3;
    background: var(--ink);
    border-color: var(--ink);
    font-weight: 600;
    color: var(--bg);
  }
  .act.primary:hover:not(:disabled) {
    filter: brightness(1.15);
    background: var(--ink);
  }
  .act:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .no-results {
    text-align: center;
    padding: 90px 0;
    color: var(--muted);
  }
  .nr-title {
    font-size: 15px;
    font-weight: 500;
    color: var(--ink2);
  }
  .nr-sub {
    font-size: 12px;
    margin-top: 6px;
  }

  /* modals */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(28, 26, 22, 0.32);
    backdrop-filter: blur(2px);
    z-index: 40;
    border: none;
    cursor: default;
  }
  .modal {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 400px;
    max-width: 92vw;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 16px;
    box-shadow: 0 40px 90px -30px rgba(40, 34, 22, 0.6);
    z-index: 41;
    overflow: hidden;
  }
  .modal-top {
    padding: 20px 22px 16px;
    border-bottom: 1px solid var(--border);
  }
  .modal-title {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .modal-sub {
    font-size: 11.5px;
    color: var(--muted);
    margin-top: 4px;
  }
  .modal-body {
    padding: 18px 22px 22px;
  }
  .field-label {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--ink2);
    margin-bottom: 7px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .field {
    width: 100%;
    box-sizing: border-box;
    height: 40px;
    padding: 0 12px;
    background: var(--surface2);
    border: 1px solid var(--border2);
    border-radius: 9px;
    font-size: 12px;
    color: var(--ink);
    outline: none;
    margin-bottom: 16px;
  }
  .field:focus {
    border-color: var(--accent);
  }
  .loc-row {
    display: flex;
    gap: 8px;
    margin-bottom: 6px;
  }
  .loc-path {
    flex: 1;
    display: flex;
    align-items: center;
    padding: 0 12px;
    height: 40px;
    background: var(--surface2);
    border: 1px solid var(--border2);
    border-radius: 9px;
    font-size: 12px;
    color: var(--ink2);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .browse {
    padding: 0 14px;
    height: 40px;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 9px;
    font: 500 12px "Space Grotesk", sans-serif;
    color: var(--ink2);
    cursor: pointer;
  }
  .browse:hover {
    background: var(--surface2);
  }
  .field-hint {
    font-size: 10.5px;
    color: var(--muted);
    margin-bottom: 16px;
  }
  .modal-actions {
    display: flex;
    gap: 9px;
    justify-content: flex-end;
  }
  .cancel {
    padding: 0 16px;
    height: 40px;
    background: none;
    border: 1px solid var(--border2);
    border-radius: 9px;
    font: 500 13px "Space Grotesk", sans-serif;
    color: var(--ink2);
    cursor: pointer;
  }
  .cancel:hover {
    background: var(--surface2);
  }
  .cta {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 18px;
    height: 40px;
    background: var(--accent);
    border: none;
    border-radius: 9px;
    font: 600 13px "Space Grotesk", sans-serif;
    color: #fff;
    cursor: pointer;
    box-shadow: 0 6px 16px -6px var(--accent);
  }
  .cta:hover:not(:disabled) {
    filter: brightness(1.07);
  }
  .cta:disabled {
    opacity: 0.55;
    cursor: default;
  }
</style>
