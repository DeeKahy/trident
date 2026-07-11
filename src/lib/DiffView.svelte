<script lang="ts">
  let { diff, emptyMessage = "No changes" }: { diff: string; emptyMessage?: string } = $props();

  type Row = { ln: string; text: string; kind: "add" | "del" | "ctx" | "hunk" | "file" };

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

  let rows = $derived(diff.trim() === "" ? [] : parse(diff));
</script>

{#if rows.length === 0}
  <div class="empty">{emptyMessage}</div>
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
</style>
