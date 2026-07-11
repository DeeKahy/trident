mod commands;
mod git;
mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(watcher::WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_repo,
            commands::git_status,
            commands::git_log,
            commands::git_branches,
            commands::git_diff_file,
            commands::git_commit_diff,
            commands::git_stage_file,
            commands::git_unstage_file,
            commands::git_stage_all,
            commands::git_unstage_all,
            commands::git_discard_file,
            commands::git_commit,
            commands::git_create_branch,
            commands::git_switch_branch,
            commands::git_delete_branch,
            commands::git_fetch,
            commands::git_pull,
            commands::git_push,
            commands::git_commit_details,
            commands::git_untracked_lines,
            commands::git_tags,
            commands::git_create_tag,
            commands::git_stash_list,
            commands::git_stash_all,
            commands::git_add_ignore,
            commands::git_remove_ignore,
            commands::git_update_merge,
            commands::git_update_rebase,
            commands::git_publish_branch,
            commands::git_reword_head,
            commands::git_undo_last,
            commands::git_revert,
            commands::git_switch_detached,
            watcher::watch_repo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
