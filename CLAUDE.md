# CLAUDE.md

Trident is a fast, free, open-source desktop git client with first-class GitHub and
GitLab support: a guided, opinionated workflow in the spirit of Fork. Tauri 2 (Rust
backend) + Svelte 5 (SvelteKit, runes mode). [ROADMAP.md](ROADMAP.md) is the source
of truth for what to build next; check items off there in the same change that
completes them.

## Hard rules

- **NEVER add `Co-Authored-By`, "Generated with", or any other AI-attribution
  trailer or marker to commits, PR bodies, or code.** Commit messages are plain
  and describe the change only.
- **No AI-sounding writing, anywhere.** No em-dashes and no emoji in commits,
  docs, comments, or UI copy. Write plainly and concretely. Avoid the usual tells: "delve",
  "robust", "seamless", "comprehensive", "It's not just X, it's Y" constructions,
  and filler enthusiasm.
- **The git-logic test suite stays green.** No exceptions.
- Any code that mutates a repository ships with a unit test proving it touches
  only what it claims to touch. Repo-corrupting bugs are the one thing we can't ship.
- Don't build Phase 4+ roadmap features (merge/rebase/graph/etc.) before Phases 2-3
  are solid.

## Architecture

The backend shells out to the system `git` binary (no libgit2) and parses
machine-readable output into typed structs. All the messy string parsing lives in
one place.

- `src-tauri/src/git/`: pure git logic, no Tauri dependency, fully unit-testable.
  - `mod.rs`: process runner (`run_git`, plus `run_git_with_ok_codes` for commands
    like `diff` that exit 1 on success) and the serializable `GitError`.
  - `types.rs`: serde structs, the wire format to the frontend, `camelCase` on the wire.
  - `status.rs`: parses `git status --porcelain=v2 -z` (renames, conflicts, unborn
    branches, paths with spaces are all covered by tests).
  - `log.rs` / `branch.rs`: custom `--format` strings using ASCII unit/record
    separators (`%x1f`/`%x1e`), which can't appear in refnames or single-line subjects.
  - `diff.rs`: unified diffs as raw text (modes: worktree / staged / untracked).
  - `repo.rs`: open/validate a repo path.
  - `testutil.rs`: `TestRepo`, a temp-dir fixture isolated from the user's global git
    config (HOME + GIT_CONFIG_GLOBAL/SYSTEM overridden, pinned commit dates, signing off).
- `src-tauri/src/commands.rs`: thin async Tauri command wrappers only; deserialize,
  call the pure function, return. No logic here.
- `src-tauri/src/github.rs`: GitHub API access (ureq). Token from GITHUB_TOKEN,
  the OS keychain (keyring crate), or the gh CLI; never stored anywhere else.
- `src/lib/git.ts`: TypeScript mirrors of the Rust types + `invoke` wrappers.
  **When a struct in `types.rs` changes, update this file in the same change.**
- `src/lib/DiffView.svelte`: renders raw unified-diff text with line coloring.
- `src/routes/+page.svelte`: the whole UI for now (three panes: branches,
  changes/history, diff).

## Commands

```sh
npm run tauri dev        # run the app (hot-reloads Svelte, rebuilds Rust)
npm run check            # svelte-check + TypeScript
npm run build            # production frontend bundle
cargo test               # backend unit tests   (run from src-tauri/)
cargo clippy --all-targets  # Rust lints        (run from src-tauri/)
```

Keep `cargo clippy` warning-free and `npm run check` at zero errors.

## Conventions

- Tests live in `#[cfg(test)] mod tests` next to the code they test and build their
  own throwaway repo via `TestRepo`. Never touch a real repository in tests.
- New git operations: pure function in `src-tauri/src/git/`, tests first, then a
  thin command wrapper, register it in `lib.rs`, mirror types + wrapper in `git.ts`.
- Tauri commands are `async fn` so git subprocesses never block the main thread.
- Errors cross the boundary as `GitError` (real git stderr in `message`); the
  frontend formats unknown throws with `errorMessage()` from `git.ts`.
- Frontend is Svelte 5 runes (`$state`, `$derived`, `$props`, snippets). No legacy
  `$:` reactive statements or stores unless there's a reason.
