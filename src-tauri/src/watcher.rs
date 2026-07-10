//! Filesystem watcher: emits a `repo-changed` event to the frontend whenever
//! the open repository changes on disk, so the UI refreshes without a manual
//! reload. One watcher exists at a time; opening a repo replaces it.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, State};

use crate::git::GitError;

#[derive(Default)]
pub struct WatcherState(Mutex<Option<RecommendedWatcher>>);

/// Should a change to this path trigger a refresh?
///
/// Everything outside `.git` counts (worktree edits). Inside `.git`, only the
/// signals of real repository changes count: HEAD (branch switch), refs
/// (commits, branches), and the index (staging) - not lock files, not the
/// object store filling up, not reflogs.
fn is_relevant(path: &Path) -> bool {
    let mut components = path.components().map(|c| c.as_os_str().to_string_lossy());
    let Some(git_pos) = components.position(|c| c == ".git") else {
        return true; // worktree file
    };
    let _ = git_pos;

    // Path inside .git: keep only HEAD, index, and refs/**, and never locks.
    let rest: PathBuf = path
        .components()
        .skip_while(|c| c.as_os_str() != ".git")
        .skip(1)
        .collect();
    let rest_str = rest.to_string_lossy();
    if rest_str.ends_with(".lock") {
        return false;
    }
    rest_str == "HEAD" || rest_str == "index" || rest_str.starts_with("refs")
}

/// Start watching `repo_path`, replacing any previous watcher.
#[tauri::command]
pub async fn watch_repo(
    app: AppHandle,
    state: State<'_, WatcherState>,
    repo_path: String,
) -> Result<(), GitError> {
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        let Ok(event) = res else { return };
        if event.paths.iter().any(|p| is_relevant(p)) {
            // Frontend debounces; failure to emit only means no auto-refresh.
            let _ = app.emit("repo-changed", ());
        }
    })
    .map_err(|e| GitError {
        message: format!("could not create filesystem watcher: {e}"),
        command: String::new(),
        exit_code: None,
    })?;

    watcher
        .watch(Path::new(&repo_path), RecursiveMode::Recursive)
        .map_err(|e| GitError {
            message: format!("could not watch {repo_path}: {e}"),
            command: String::new(),
            exit_code: None,
        })?;

    *state.0.lock().expect("watcher lock") = Some(watcher);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_paths_are_relevant() {
        assert!(is_relevant(Path::new("/repo/src/main.rs")));
        assert!(is_relevant(Path::new("/repo/README.md")));
    }

    #[test]
    fn git_internals_are_filtered_to_real_signals() {
        assert!(is_relevant(Path::new("/repo/.git/HEAD")));
        assert!(is_relevant(Path::new("/repo/.git/index")));
        assert!(is_relevant(Path::new("/repo/.git/refs/heads/main")));

        assert!(!is_relevant(Path::new("/repo/.git/index.lock")));
        assert!(!is_relevant(Path::new("/repo/.git/refs/heads/main.lock")));
        assert!(!is_relevant(Path::new("/repo/.git/objects/ab/cdef123")));
        assert!(!is_relevant(Path::new("/repo/.git/logs/HEAD")));
        assert!(!is_relevant(Path::new("/repo/.git/FETCH_HEAD")));
    }
}
