# Trident Roadmap

Trident is a fast, free, open-source git client with first-class GitHub and GitLab
support — a guided, opinionated workflow in the spirit of Fork, built with Tauri
(Rust backend) and Svelte.

**Guiding principle:** build in layers where each layer is independently usable and
testable, so there is always a working app and never a six-week stretch of broken
half-app. Advanced features wait until the foundation is genuinely solid — advanced
operations on a shaky foundation is how you get data-loss bugs.

## How to track this

This file is the source of truth. To track it on GitHub:

1. Create one **milestone per phase** (`gh api` or the web UI).
2. Copy each phase's checklist into a **tracking issue** for that milestone —
   task-list checkboxes are clickable in issues, and GitHub shows progress
   (`- [ ]` items) on the issue automatically.
3. Optionally add the tracking issues to a **GitHub Project** board with
   Todo / In progress / Done columns.
4. Check items off here in the same PR that completes them, so the file never lies.

---

## Phase 0 — Spike the architecture (done)

Prove the risky part: Rust↔frontend communication, git process spawning, and the
dev loop working together as one vertical slice.

- [x] Install Rust, Node, and Tauri prerequisites
- [x] Scaffold Tauri + Svelte project (`npm create tauri-app`)
- [x] Rust command that shells out to `git status`, returns structured data
- [x] Svelte component that renders it
- [x] Confirm the dev loop feels good: `npm run tauri dev` hot-reloads the frontend
      and rebuilds Rust on change

## Phase 1 — Git backend as a standalone Rust layer

A clean Rust module wrapping every git operation, exposed as typed Tauri commands
with `serde` structs. All porcelain string-parsing lives in one place
(`src-tauri/src/git/`). **Test-first**: every function gets a unit test against a
throwaway temp repo. That suite is the safety net for everything after — git bugs
that corrupt a repo are the one category we can't ship.

Read operations:
- [x] `open_repo` — validate a path, resolve work-tree root, detect detached HEAD
- [x] `git_status` — porcelain v2 `-z` parsed into staged/unstaged/untracked/conflicted
- [x] `git_log` — history with hash, author, date, parents, subject (limit + skip)
- [x] `git_branches` — local + remote, HEAD flag, upstream, ahead/behind
- [x] `git_diff_file` — unified diff for worktree / staged / untracked modes
- [x] `git_commit_diff` — full patch for a commit

Write operations (backend only; UI comes in Phase 3):
- [x] Stage / unstage a file (and `stage_all` / `unstage_all`)
- [x] Discard changes in a file (with the test proving it only touches that file)
- [x] Commit (message, amend flag)
- [x] Create / switch / delete branch
- [x] Fetch, pull, push
- [ ] Structured errors for every failure mode a UI needs to explain

Infrastructure:
- [x] `GitError` type that crosses the Tauri boundary with the real git message
- [x] Temp-repo test harness isolated from user git config (signing, hooks, defaults)
- [x] CI running `cargo test` + `cargo clippy` + `svelte-check` on every push

## Phase 2 — A read-only viewer

Wire the backend to UI, but only the parts that read. A functioning git viewer
that literally cannot damage a repository — dogfood it on real projects immediately.

- [x] Open / pick a repository (native folder dialog + manual path)
- [x] Staged / unstaged / untracked / conflicted file lists
- [x] Commit history as a plain vertical list (fancy lanes come in Phase 4)
- [x] Diff view for the selected file or commit
- [x] Remember recently opened repositories across launches
- [x] Auto-refresh when the repo changes on disk (filesystem watcher)
- [x] Commit details panel (full message, author/committer, changed-file list)
- [x] Load more history on scroll (the backend already supports limit/skip)

## Phase 3 — Write operations and the "set flow"

The opinionated workflow gets designed here rather than inherited. Sketch the
guided commit→sync loop on paper first, then implement. The moment this works,
**develop Trident using Trident.**

- [x] Sketch the guided commit→sync flow (defaults, guardrails, what makes it easier than Fork)
- [x] Stage / unstage from the UI (per-file; per-hunk can wait)
- [x] Discard changes with a real confirmation (destructive — design the guardrail)
- [x] Commit UI: message box, subject/body split, amend
- [x] Branch create / switch / delete
- [x] Fetch / pull / push buttons with progress and honest error reporting
- [x] The guided sync loop: one obvious "what do I do next" path (smart Sync button:
      fetch, then pull, push, or offer rebase/merge on divergence)
- [ ] Switch daily driving to Trident (the dogfooding milestone)

## Phase 4 — The genuinely hard parts

Each of these is a mini-project. They most define whether it "feels like Fork."

- [ ] Merge (with sensible conflict detection and messaging)
- [ ] Conflict-resolution UI (ours/theirs/both, per-conflict, mark resolved)
- [ ] Rebase, then interactive rebase (reorder, squash, edit, drop)
- [ ] Cherry-pick
- [ ] Stash (save, list, apply, pop, drop)
- [ ] Real commit graph: proper lane layout algorithm
- [ ] List virtualization so the graph stays smooth over thousands of commits
- [ ] Remote authentication via git credential helpers (HTTPS + SSH agent)

## Phase 5 — GitHub & GitLab integration

The reason Trident exists: first-class GitHub and GitLab support, not just plain git remotes.

- [ ] OAuth device-flow sign-in for GitHub and GitLab (tokens in OS keychain)
- [ ] Detect the hosting service from the remote URL (github.com, gitlab.com, self-hosted GitLab)
- [ ] Pull requests / merge requests: list, view, check out a PR/MR branch locally
- [ ] Create a PR/MR from the current branch (the natural end of the guided sync loop)
- [ ] CI status checks shown on branches and commits
- [ ] Clone from a list of your repositories

## Phase 6 — Polish

- [ ] Keyboard shortcuts for the whole core loop
- [ ] Empty states and error recovery everywhere
- [ ] Light / dark theming
- [ ] Visual density and refinement pass (the thing that makes Fork pleasant)
- [ ] Performance pass on large repos (status/log latency, diff rendering)
- [ ] Final tuning of the guided-workflow personality

## Phase 7 — Packaging and distribution

- [ ] Decide: bundle git vs. detect system install (start with detection + a clear
      error when missing; revisit bundling later)
- [ ] `tauri build` installers for Windows, macOS, Linux
- [ ] GitHub Actions building all three on every release tag
- [ ] **Code signing**: Apple notarization + Windows signing (budget real time —
      tedious, poorly documented, and it gates scary-warning-free installs)
- [ ] Tauri auto-updater
- [ ] First public release (Phases 0–3 = the "minimum lovable" version)

## Phase 8 — Open-source hygiene

Being open and free is the whole point, so do it properly.

- [ ] Pick a license (MIT for maximum adoption vs. GPL to keep derivatives open)
- [ ] Real README: what it is, screenshots, install, build-from-source
- [ ] CONTRIBUTING guide and issue templates
- [ ] CI running Rust tests and frontend checks on every PR (if not done in Phase 1)

---

## Rules that run across all phases

- **Version-control the project from commit one.** Dogfood the instant it can commit.
- **The git-logic test suite stays green.** Hard rule, no exceptions.
- **No Phase 4+ features before Phases 2–3 are genuinely solid.**
- Repository-mutating code never ships without a test that proves it touches only
  what it claims to touch.

## Milestones

| Milestone | Meaning |
|---|---|
| Phases 0–3 done | Minimum lovable: a client you'd use daily; good first public release |
| Phases 4–6 done | A real Fork competitor |
| Phase 7 done | Anyone can actually install and run it |
