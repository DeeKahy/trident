mod commands;
mod git;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
