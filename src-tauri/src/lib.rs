mod commands;
mod git;
mod github;
mod shell;
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
            commands::repo_summary,
            commands::clone_repo,
            commands::init_repo,
            commands::scan_repos,
            commands::scan_folder,
            commands::code_stats,
            github::github_account,
            github::github_repos,
            github::github_connect,
            github::github_disconnect,
            shell::open_in_editor,
            shell::open_in_terminal,
            watcher::watch_repo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
