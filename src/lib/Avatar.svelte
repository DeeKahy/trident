<script lang="ts">
  let { email, name, size = 20 }: { email: string; name: string; size?: number } = $props();

  let url = $state<string | null>(null);
  let failed = $state(false);

  let initials = $derived(
    name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0].toUpperCase())
      .join("") || "?"
  );

  // Deterministic hue from the email so each person keeps their color.
  let hue = $derived([...email].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7));

  $effect(() => {
    failed = false;
    url = null;

    // GitHub noreply addresses identify the account directly:
    // "12345+user@users.noreply.github.com" or "user@users.noreply.github.com".
    const gh = email.match(/^(?:(\d+)\+)?([a-z0-9-]+)@users\.noreply\.github\.com$/i);
    if (gh) {
      url = gh[1]
        ? `https://avatars.githubusercontent.com/u/${gh[1]}?s=${size * 2}`
        : `https://github.com/${gh[2]}.png?size=${size * 2}`;
      return;
    }

    // Anything else: Gravatar (used by both GitHub and GitLab accounts that
    // registered one), falling back to initials when there is no avatar.
    let cancelled = false;
    (async () => {
      const bytes = new TextEncoder().encode(email.trim().toLowerCase());
      const digest = await crypto.subtle.digest("SHA-256", bytes);
      const hex = [...new Uint8Array(digest)]
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");
      if (!cancelled) url = `https://gravatar.com/avatar/${hex}?s=${size * 2}&d=404`;
    })();
    return () => {
      cancelled = true;
    };
  });
</script>

{#if url && !failed}
  <img
    class="avatar"
    src={url}
    alt={name}
    style="width:{size}px;height:{size}px"
    loading="lazy"
    onerror={() => (failed = true)}
  />
{:else}
  <span
    class="avatar fallback"
    style="width:{size}px;height:{size}px;font-size:{Math.round(size * 0.42)}px;background:hsl({hue},45%,32%)"
  >
    {initials}
  </span>
{/if}

<style>
  .avatar {
    border-radius: 50%;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    object-fit: cover;
  }
  .fallback {
    color: #e6edf3;
    font-weight: 600;
    letter-spacing: 0.02em;
    user-select: none;
  }
</style>
