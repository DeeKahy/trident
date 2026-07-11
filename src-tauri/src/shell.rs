//! Launch external tools on a repository folder: the "VS Code" and
//! terminal buttons on project cards.

use std::path::Path;
use std::process::Command;

use crate::git::GitError;

fn spawn(mut cmd: Command, what: &str) -> Result<(), GitError> {
    cmd.spawn().map(|_| ()).map_err(|e| GitError {
        message: format!("could not open {what}: {e}"),
        command: String::new(),
        exit_code: None,
    })
}

#[tauri::command]
pub async fn open_in_editor(path: String) -> Result<(), GitError> {
    if !Path::new(&path).is_dir() {
        return Err(GitError {
            message: format!("folder not found: {path}"),
            command: String::new(),
            exit_code: None,
        });
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("open");
        cmd.args(["-a", "Visual Studio Code", &path]);
        spawn(cmd, "VS Code")
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut cmd = Command::new("code");
        cmd.arg(&path);
        spawn(cmd, "VS Code")
    }
}

#[tauri::command]
pub async fn open_in_terminal(path: String) -> Result<(), GitError> {
    if !Path::new(&path).is_dir() {
        return Err(GitError {
            message: format!("folder not found: {path}"),
            command: String::new(),
            exit_code: None,
        });
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("open");
        cmd.args(["-a", "Terminal", &path]);
        spawn(cmd, "Terminal")
    }
    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("x-terminal-emulator");
        cmd.current_dir(&path);
        spawn(cmd, "a terminal")
    }
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "cmd", "/K", "cd", "/d", &path]);
        spawn(cmd, "a terminal")
    }
}
