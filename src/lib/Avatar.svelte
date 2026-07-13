<script lang="ts" module>
  // Resolve each identity's avatar at most once and remember the outcome, so the
  // history view re-rendering (or scrolling avatars in and out of view) never
  // re-hits the network or re-flashes. A cached `null` means "no avatar, use
  // initials" — the common case for authors whose email has no Gravatar.
  const cache = new Map<string, string | null>();
  const inflight = new Map<string, Promise<string | null>>();

  function githubUrl(email: string, px: number): string | null {
    // GitHub noreply addresses identify the account directly:
    // "12345+user@users.noreply.github.com" or "user@users.noreply.github.com".
    const gh = email.match(/^(?:(\d+)\+)?([a-z0-9-]+)@users\.noreply\.github\.com$/i);
    if (!gh) return null;
    return gh[1]
      ? `https://avatars.githubusercontent.com/u/${gh[1]}?s=${px}`
      : `https://github.com/${gh[2]}.png?size=${px}`;
  }

  async function gravatarUrl(email: string, px: number): Promise<string> {
    // Gravatar, used by both GitHub and GitLab accounts that registered one.
    // d=404 so a missing avatar is a load error we can fall back to initials on.
    const bytes = new TextEncoder().encode(email.trim().toLowerCase());
    const digest = await crypto.subtle.digest("SHA-256", bytes);
    const hex = [...new Uint8Array(digest)]
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    return `https://gravatar.com/avatar/${hex}?s=${px}&d=404`;
  }

  // Load the candidate off-DOM so a missing avatar (Gravatar 404) never appears
  // as a broken <img> that flashes to initials — we only ever mount an <img>
  // once it is confirmed to load.
  function verify(src: string): Promise<boolean> {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => resolve(true);
      img.onerror = () => resolve(false);
      img.src = src;
    });
  }

  // Returns the avatar URL to show, or null to fall back to initials. Cached and
  // de-duplicated per identity so repeated renders are free.
  function resolveAvatar(email: string, px: number): Promise<string | null> {
    const key = `${email.trim().toLowerCase()}|${px}`;
    const cached = cache.get(key);
    if (cached !== undefined) return Promise.resolve(cached);
    const existing = inflight.get(key);
    if (existing) return existing;

    const work = (async () => {
      const candidate = githubUrl(email, px) ?? (await gravatarUrl(email, px));
      const resolved = (await verify(candidate)) ? candidate : null;
      cache.set(key, resolved);
      inflight.delete(key);
      return resolved;
    })();
    inflight.set(key, work);
    return work;
  }
</script>

<script lang="ts">
  let { email, name, size = 20 }: { email: string; name: string; size?: number } = $props();

  let url = $state<string | null>(null);

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
    const px = size * 2;
    // Show initials until (and unless) a real avatar is confirmed to load.
    url = null;
    let cancelled = false;
    resolveAvatar(email, px).then((resolved) => {
      if (!cancelled) url = resolved;
    });
    return () => {
      cancelled = true;
    };
  });
</script>

{#if url}
  <img
    class="avatar"
    src={url}
    alt={name}
    style="width:{size}px;height:{size}px"
    loading="lazy"
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
