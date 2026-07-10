<script lang="ts">
  let { diff, emptyMessage = "No changes" }: { diff: string; emptyMessage?: string } = $props();

  type LineKind = "add" | "del" | "hunk" | "meta" | "context";

  function classify(line: string): LineKind {
    if (line.startsWith("+++") || line.startsWith("---")) return "meta";
    if (line.startsWith("@@")) return "hunk";
    if (line.startsWith("+")) return "add";
    if (line.startsWith("-")) return "del";
    if (
      line.startsWith("diff --git") ||
      line.startsWith("index ") ||
      line.startsWith("new file") ||
      line.startsWith("deleted file") ||
      line.startsWith("rename ") ||
      line.startsWith("similarity ")
    ) {
      return "meta";
    }
    return "context";
  }

  let lines = $derived(
    diff
      .replace(/\n$/, "")
      .split("\n")
      .map((text) => ({ text, kind: classify(text) }))
  );
</script>

{#if diff.trim() === ""}
  <div class="empty">{emptyMessage}</div>
{:else}
  <pre class="diff">{#each lines as line}<span class="line {line.kind}">{line.text || " "}
</span>{/each}</pre>
{/if}

<style>
  .diff {
    margin: 0;
    padding: 0.75rem;
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 12px;
    line-height: 1.5;
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
  }
  .line {
    display: block;
    white-space: pre;
    padding: 0 0.5rem;
  }
  .line.add {
    background: rgba(63, 185, 80, 0.15);
    color: var(--diff-add, #3fb950);
  }
  .line.del {
    background: rgba(248, 81, 73, 0.15);
    color: var(--diff-del, #f85149);
  }
  .line.hunk {
    color: var(--diff-hunk, #58a6ff);
    background: rgba(88, 166, 255, 0.08);
  }
  .line.meta {
    color: var(--fg-muted, #8b949e);
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--fg-muted, #8b949e);
    font-size: 13px;
  }
</style>
