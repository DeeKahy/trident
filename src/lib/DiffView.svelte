<script lang="ts">
  // `collapsible` is opt-in (the expanded modal uses it): it splits the diff
  // into per-file sections you can minimize/maximize and add/remove from view.
  // The inline diff leaves it off and renders one flat list as before.
  let {
    diff,
    emptyMessage = "No changes",
    collapsible = false,
  }: { diff: string; emptyMessage?: string; collapsible?: boolean } = $props();

  type Row = { ln: string; text: string; kind: "add" | "del" | "ctx" | "hunk" | "file" };
  type FileGroup = { path: string; rows: Row[]; add: number; del: number };

  // Turn raw unified-diff text into numbered rows: deletions carry the old
  // line number, additions and context the new one.
  function parse(raw: string): Row[] {
    const rows: Row[] = [];
    let oldLn = 0;
    let newLn = 0;
    for (const line of raw.replace(/\n$/, "").split("\n")) {
      if (line.startsWith("diff --git")) {
        const m = line.match(/ b\/(.*)$/);
        rows.push({ ln: "", text: m ? m[1] : line, kind: "file" });
      } else if (line.startsWith("@@")) {
        const m = line.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/);
        if (m) {
          oldLn = parseInt(m[1]);
          newLn = parseInt(m[2]);
          rows.push({ ln: "", text: `⋯${m[3] || ""}`, kind: "hunk" });
        }
      } else if (
        line.startsWith("index ") ||
        line.startsWith("+++") ||
        line.startsWith("---") ||
        line.startsWith("new file") ||
        line.startsWith("deleted file") ||
        line.startsWith("similarity") ||
        line.startsWith("rename") ||
        line.startsWith("old mode") ||
        line.startsWith("new mode") ||
        line.startsWith("Binary files") ||
        line.startsWith("\\ No newline")
      ) {
        continue;
      } else if (line.startsWith("+")) {
        rows.push({ ln: String(newLn++), text: line.slice(1), kind: "add" });
      } else if (line.startsWith("-")) {
        rows.push({ ln: String(oldLn++), text: line.slice(1), kind: "del" });
      } else {
        rows.push({ ln: String(newLn++), text: line.startsWith(" ") ? line.slice(1) : line, kind: "ctx" });
        oldLn++;
      }
    }
    return rows;
  }

  // Split the flat rows into one group per file, tallying +/- for the header.
  function groupByFile(rows: Row[]): FileGroup[] {
    const groups: FileGroup[] = [];
    let cur: FileGroup | null = null;
    for (const row of rows) {
      if (row.kind === "file") {
        cur = { path: row.text, rows: [], add: 0, del: 0 };
        groups.push(cur);
      } else {
        if (!cur) {
          cur = { path: "", rows: [], add: 0, del: 0 };
          groups.push(cur);
        }
        cur.rows.push(row);
        if (row.kind === "add") cur.add++;
        else if (row.kind === "del") cur.del++;
      }
    }
    return groups;
  }

  let rows = $derived(diff.trim() === "" ? [] : parse(diff));
  let files = $derived(collapsible ? groupByFile(rows) : []);

  // Per-file view state, keyed by path so it survives a re-parse (e.g. when the
  // modal reloads the same files with full context).
  let collapsed = $state<Record<string, boolean>>({});
  let removed = $state<Record<string, boolean>>({});

  let shown = $derived(files.filter((f) => !removed[f.path]));
  let removedPaths = $derived(files.filter((f) => removed[f.path]).map((f) => f.path));

  function expandAll() {
    collapsed = {};
  }
  function collapseAll() {
    const next: Record<string, boolean> = {};
    for (const f of files) next[f.path] = true;
    collapsed = next;
  }
</script>

{#if rows.length === 0}
  <div class="empty">{emptyMessage}</div>
{:else if collapsible}
  <div class="groups">
    <div class="toolbar mono">
      <span class="count">{shown.length}/{files.length} file{files.length === 1 ? "" : "s"}</span>
      <span class="spacer"></span>
      <button class="tool" onclick={expandAll}>Expand all</button>
      <button class="tool" onclick={collapseAll}>Collapse all</button>
    </div>

    {#each shown as f (f.path)}
      <div class="group" class:collapsed={collapsed[f.path]}>
        <div class="group-head">
          <button
            class="head-toggle mono"
            onclick={() => (collapsed[f.path] = !collapsed[f.path])}
            aria-expanded={!collapsed[f.path]}
          >
            <span class="caret" class:open={!collapsed[f.path]}>▶</span>
            <span class="path">{f.path || "(diff)"}</span>
          </button>
          <span class="pill add">+{f.add}</span>
          <span class="pill del">-{f.del}</span>
          <button class="remove" title="Remove from view" onclick={() => (removed[f.path] = true)}>×</button>
        </div>
        {#if !collapsed[f.path]}
          <div class="diff mono">
            {#each f.rows as row, i (i)}
              <div class="line {row.kind}">
                <span class="ln">{row.ln}</span><span class="text">{row.text || " "}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}

    {#if removedPaths.length > 0}
      <div class="restore-strip">
        <span class="mono tiny">removed from view:</span>
        {#each removedPaths as p (p)}
          <span class="mono tiny chip">
            {p}
            <button class="restore-btn" onclick={() => (removed[p] = false)}>add back</button>
          </span>
        {/each}
      </div>
    {/if}
  </div>
{:else}
  <div class="diff mono">
    {#each rows as row, i (i)}
      <div class="line {row.kind}">
        <span class="ln">{row.ln}</span><span class="text">{row.text || " "}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .diff {
    font-size: 12px;
    line-height: 1.75;
    padding: 8px 0;
  }
  .mono {
    font-family: "JetBrains Mono", ui-monospace, monospace;
  }
  .line {
    display: flex;
    padding: 0 6px;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .ln {
    display: inline-block;
    width: 38px;
    text-align: right;
    padding-right: 14px;
    color: var(--muted);
    opacity: 0.65;
    user-select: none;
    flex: none;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .line.ctx {
    color: var(--ink2);
  }
  .line.add {
    background: var(--add-soft);
    color: var(--add);
  }
  .line.del {
    background: var(--del-soft);
    color: var(--del);
  }
  .line.hunk {
    color: var(--muted);
    padding-top: 4px;
    padding-bottom: 4px;
  }
  .line.file {
    color: var(--ink);
    font-weight: 700;
    padding-top: 10px;
    padding-bottom: 2px;
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 120px;
    color: var(--muted);
    font-size: 13px;
    font-family: "Space Grotesk", system-ui, sans-serif;
  }

  /* Collapsible per-file view (expanded modal only). */
  .groups {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--muted);
    position: sticky;
    top: 0;
    background: var(--surface2, var(--bg));
    padding: 6px 2px;
    z-index: 1;
  }
  .toolbar .count {
    letter-spacing: 0.02em;
  }
  .spacer {
    flex: 1;
  }
  .tool {
    background: none;
    border: 1px solid var(--line, rgba(128, 128, 128, 0.3));
    color: var(--ink2);
    border-radius: 6px;
    padding: 2px 8px;
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }
  .tool:hover {
    color: var(--ink);
    border-color: var(--accent);
  }
  .group {
    border: 1px solid var(--line, rgba(128, 128, 128, 0.22));
    border-radius: 8px;
    overflow: hidden;
  }
  .group-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    background: var(--panel-2, rgba(128, 128, 128, 0.08));
  }
  .group.collapsed .group-head {
    background: transparent;
  }
  .head-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    color: var(--ink);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .caret {
    flex: none;
    font-size: 9px;
    color: var(--muted);
    transition: transform 0.12s ease;
  }
  .caret.open {
    transform: rotate(90deg);
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pill {
    flex: none;
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 5px;
  }
  .pill.add {
    color: var(--add);
    background: var(--add-soft);
  }
  .pill.del {
    color: var(--del);
    background: var(--del-soft);
  }
  .remove {
    flex: none;
    background: none;
    border: none;
    color: var(--muted);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    padding: 0 2px;
  }
  .remove:hover {
    color: var(--del);
  }
  .restore-strip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 6px 2px;
    color: var(--muted);
  }
  .tiny {
    font-size: 11px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px;
    border: 1px solid var(--line, rgba(128, 128, 128, 0.3));
    border-radius: 6px;
  }
  .restore-btn {
    background: none;
    border: none;
    color: var(--accent);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
  }
</style>
