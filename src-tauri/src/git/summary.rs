//! Project-card summary: one call that tells the hub everything it shows
//! about a repository, including a language breakdown of tracked files.

use std::collections::HashMap;
use std::path::Path;

use super::types::{LangStat, RepoSummary};
use super::{log, repo, run_git, status, Result};

/// Map a file to a display language. Docs and config formats return None
/// so lock files and READMEs don't drown out the actual code.
fn language_of(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    if lower.ends_with("dockerfile") {
        return Some("Docker");
    }
    let ext = lower.rsplit('.').next()?;
    Some(match ext {
        "rs" => "Rust",
        "ts" | "tsx" | "mts" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "svelte" => "Svelte",
        "vue" => "Vue",
        "py" => "Python",
        "rb" => "Ruby",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "swift" => "Swift",
        "c" | "h" => "C",
        "cc" | "cpp" | "cxx" | "hpp" => "C++",
        "cs" => "C#",
        "php" => "PHP",
        "ex" | "exs" => "Elixir",
        "zig" => "Zig",
        "lua" => "Lua",
        "css" | "scss" | "sass" | "less" => "CSS",
        "html" | "htm" => "HTML",
        "sh" | "bash" | "zsh" => "Shell",
        "sql" => "SQL",
        "astro" => "Astro",
        "ipynb" => "Jupyter",
        _ => return None,
    })
}

/// Language shares by tracked byte size, largest first, top three.
fn languages(repo_path: &Path) -> Result<Vec<LangStat>> {
    let raw = run_git(repo_path, &["ls-files", "-z"])?;
    let mut bytes: HashMap<&'static str, u64> = HashMap::new();
    for file in raw.split('\0').filter(|p| !p.is_empty()) {
        let Some(lang) = language_of(file) else {
            continue;
        };
        let size = std::fs::metadata(repo_path.join(file))
            .map(|m| m.len())
            .unwrap_or(0);
        *bytes.entry(lang).or_insert(0) += size;
    }
    let total: u64 = bytes.values().sum();
    if total == 0 {
        return Ok(Vec::new());
    }
    let mut list: Vec<(&str, u64)> = bytes.into_iter().collect();
    list.sort_by_key(|&(_, b)| std::cmp::Reverse(b));
    list.truncate(3);
    let top_total: u64 = list.iter().map(|(_, b)| b).sum();
    Ok(list
        .into_iter()
        .map(|(name, b)| LangStat {
            name: name.to_string(),
            pct: ((b as f64 / top_total as f64) * 100.0).round() as u32,
        })
        .collect())
}

/// Everything the hub's project card needs, in one call.
pub fn repo_summary(path: &Path) -> Result<RepoSummary> {
    let info = repo::open_repo(path)?;
    let root = Path::new(&info.path);
    let s = status::status(root)?;
    let changes =
        (s.staged.len() + s.unstaged.len() + s.untracked.len() + s.conflicted.len()) as u32;
    let last_commit_date = log::log(root, 1, 0, false)?.first().map(|c| c.date.clone());
    let origin_url = run_git(root, &["remote", "get-url", "origin"])
        .ok()
        .map(|u| u.trim().to_string());

    Ok(RepoSummary {
        name: info.name,
        path: info.path.clone(),
        branch: info.head,
        changes,
        ahead: s.branch.ahead,
        behind: s.branch.behind,
        last_commit_date,
        origin_url,
        langs: languages(root)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;

    #[test]
    fn summary_reports_branch_changes_and_languages() {
        let repo = TestRepo::with_initial_commit();
        repo.write("src/main.rs", &"fn main() {}\n".repeat(30));
        repo.write("src/app.ts", &"export {}\n".repeat(10));
        repo.git(&["add", "."]);
        repo.commit("add code");
        repo.write("dirty.txt", "x\n");

        let s = repo_summary(repo.path()).unwrap();
        assert_eq!(s.branch, "main");
        assert_eq!(s.changes, 1);
        assert!(s.last_commit_date.is_some());
        assert_eq!(s.langs[0].name, "Rust");
        assert!(s.langs.iter().any(|l| l.name == "TypeScript"));
        let total: u32 = s.langs.iter().map(|l| l.pct).sum();
        assert!((98..=102).contains(&total), "pcts sum to ~100, got {total}");
    }

    #[test]
    fn docs_and_config_do_not_count_as_languages() {
        let repo = TestRepo::with_initial_commit(); // only README.md
        let s = repo_summary(repo.path()).unwrap();
        assert!(s.langs.is_empty());
    }

    #[test]
    fn summary_rejects_a_non_repo() {
        let dir = tempfile::tempdir().unwrap();
        assert!(repo_summary(dir.path()).is_err());
    }
}
