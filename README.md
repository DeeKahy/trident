# Trident

A fast, free, open-source git client with first-class GitHub and GitLab support.
Guided, opinionated workflow in the spirit of [Fork](https://git-fork.com/) — built
with [Tauri](https://tauri.app) (Rust backend) and [Svelte](https://svelte.dev).

**Status: early spike.** The read-only viewer works: open a repo, browse
staged/unstaged/untracked changes, commit history, branches, and diffs. Nothing can
mutate a repository yet — see [ROADMAP.md](ROADMAP.md) for where this is going.

## Install

macOS (Apple Silicon):

```sh
brew install --cask --no-quarantine deekahy/tap/trident
```

Updates arrive with `brew upgrade`. The `--no-quarantine` flag matters: the
app is not code-signed yet, and without it macOS refuses the first launch
with a misleading "trident is damaged" message. An already-installed copy is
fixed with `xattr -dr com.apple.quarantine /Applications/trident.app`.

Installers for Windows (.msi/.exe) and Linux (.deb/.rpm/.AppImage) are on the
[releases page](https://github.com/DeeKahy/trident/releases); Windows shows a
SmartScreen warning for the same unsigned reason.

## Architecture

- `src-tauri/src/git/` — the git backend: plain Rust functions that shell out to the
  system `git`, parse machine-readable output (`--porcelain=v2 -z`, custom `--format`
  strings) into typed `serde` structs, and know nothing about Tauri. Fully unit-tested
  against throwaway temp repos.
- `src-tauri/src/commands.rs` — thin async Tauri command wrappers around the git module.
- `src/lib/git.ts` — typed TypeScript mirrors of the Rust structs and `invoke` wrappers.
- `src/routes/+page.svelte` — the app UI (Svelte 5 runes).

## Development

Prerequisites: [Rust](https://rustup.rs), Node 20+, git, and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```sh
npm install
npm run tauri dev    # run the app; hot-reloads Svelte, rebuilds Rust on change
```

Checks:

```sh
cargo test           # git backend unit tests (run from src-tauri/)
cargo clippy         # Rust lints (run from src-tauri/)
npm run check        # svelte-check + TypeScript
npm run tauri build  # production installers
```

## Contributing

The one hard rule: **the git-logic test suite stays green.** Any code that mutates a
repository ships with a test proving it touches only what it claims to touch.
