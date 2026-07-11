//! GitHub integration: list the user's repositories on the hub.
//!
//! Token resolution order: the GITHUB_TOKEN environment variable, a token
//! the user pasted (stored in the OS keychain), then an existing `gh` CLI
//! login. No token means "not connected" - never an error.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::git::GitError;

const KEYCHAIN_SERVICE: &str = "trident";
const KEYCHAIN_USER: &str = "github-token";
const API: &str = "https://api.github.com";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubUser {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: String,
    /// Where the token came from: "env", "keychain", or "gh".
    pub token_source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubRepo {
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub fork: bool,
    pub html_url: String,
    pub clone_url: String,
    pub pushed_at: Option<String>,
    pub stars: u32,
    pub language: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
struct RawUser {
    login: String,
    name: Option<String>,
    avatar_url: String,
}

#[derive(Deserialize)]
struct RawRepo {
    name: String,
    full_name: String,
    private: bool,
    fork: bool,
    html_url: String,
    clone_url: String,
    pushed_at: Option<String>,
    stargazers_count: u32,
    language: Option<String>,
    description: Option<String>,
}

fn err(message: impl Into<String>) -> GitError {
    GitError {
        message: message.into(),
        command: String::new(),
        exit_code: None,
    }
}

fn keychain_entry() -> Result<keyring::Entry, GitError> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)
        .map_err(|e| err(format!("keychain unavailable: {e}")))
}

fn gh_cli_token() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

fn resolve_token() -> Option<(String, String)> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            return Some((token.trim().to_string(), "env".to_string()));
        }
    }
    if let Ok(entry) = keychain_entry() {
        if let Ok(token) = entry.get_password() {
            if !token.trim().is_empty() {
                return Some((token.trim().to_string(), "keychain".to_string()));
            }
        }
    }
    gh_cli_token().map(|t| (t, "gh".to_string()))
}

fn get(path: &str, token: &str) -> Result<serde_json::Value, GitError> {
    ureq::get(&format!("{API}{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", "trident-git-client")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(401, _) => err("GitHub rejected the token (sign in again)"),
            ureq::Error::Status(code, _) => err(format!("GitHub returned HTTP {code}")),
            other => err(format!("could not reach GitHub: {other}")),
        })?
        .into_json()
        .map_err(|e| err(format!("unexpected GitHub response: {e}")))
}

fn fetch_user(token: &str, source: &str) -> Result<GithubUser, GitError> {
    let raw: RawUser = serde_json::from_value(get("/user", token)?)
        .map_err(|e| err(format!("unexpected GitHub response: {e}")))?;
    Ok(GithubUser {
        login: raw.login,
        name: raw.name,
        avatar_url: raw.avatar_url,
        token_source: source.to_string(),
    })
}

/// The connected account, or None when no token is available anywhere.
#[tauri::command]
pub async fn github_account() -> Result<Option<GithubUser>, GitError> {
    match resolve_token() {
        None => Ok(None),
        Some((token, source)) => fetch_user(&token, &source).map(Some),
    }
}

/// Every repository the user can access, newest push first.
#[tauri::command]
pub async fn github_repos() -> Result<Vec<GithubRepo>, GitError> {
    let (token, _) = resolve_token().ok_or_else(|| err("not connected to GitHub"))?;
    let mut repos = Vec::new();
    for page in 1..=5 {
        let value = get(
            &format!("/user/repos?per_page=100&sort=pushed&page={page}"),
            &token,
        )?;
        let raw: Vec<RawRepo> = serde_json::from_value(value)
            .map_err(|e| err(format!("unexpected GitHub response: {e}")))?;
        let count = raw.len();
        repos.extend(raw.into_iter().map(|r| GithubRepo {
            name: r.name,
            full_name: r.full_name,
            private: r.private,
            fork: r.fork,
            html_url: r.html_url,
            clone_url: r.clone_url,
            pushed_at: r.pushed_at,
            stars: r.stargazers_count,
            language: r.language,
            description: r.description,
        }));
        if count < 100 {
            break;
        }
    }
    Ok(repos)
}

/// Validate a pasted token against the API, then keep it in the keychain.
#[tauri::command]
pub async fn github_connect(token: String) -> Result<GithubUser, GitError> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(err("token is empty"));
    }
    let user = fetch_user(&token, "keychain")?;
    keychain_entry()?
        .set_password(&token)
        .map_err(|e| err(format!("could not store the token: {e}")))?;
    Ok(user)
}

/// Forget the stored token. env/gh tokens are outside our control and
/// keep working; the UI explains that.
#[tauri::command]
pub async fn github_disconnect() -> Result<(), GitError> {
    if let Ok(entry) = keychain_entry() {
        let _ = entry.delete_credential();
    }
    Ok(())
}
