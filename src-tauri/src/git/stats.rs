//! Code statistics for the Stats tab: tokei line counts per language plus
//! a few facts mined from git history.

use std::path::Path;

use serde::Serialize;
use tokei::{Config, Languages};

use super::{run_git, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LangDetail {
    pub name: String,
    pub files: u32,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributor {
    pub name: String,
    pub email: String,
    pub commits: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeStats {
    pub files: u32,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
    pub commits: u32,
    pub first_commit_date: Option<String>,
    pub languages: Vec<LangDetail>,
    pub contributors: Vec<Contributor>,
}

/// Directories never worth counting even when a repo forgot to ignore them.
const EXCLUDED: &[&str] = &["node_modules", "target", "dist", "build", "vendor", ".git"];

fn count_lines(repo: &Path) -> (u32, u64, u64, u64, Vec<LangDetail>) {
    let mut languages = Languages::new();
    languages.get_statistics(&[repo], EXCLUDED, &Config::default());

    let mut list: Vec<LangDetail> = languages
        .iter()
        .filter(|(_, l)| l.code > 0)
        .map(|(t, l)| LangDetail {
            name: t.to_string(),
            files: l.reports.len() as u32,
            code: l.code as u64,
            comments: l.comments as u64,
            blanks: l.blanks as u64,
        })
        .collect();
    list.sort_by_key(|l| std::cmp::Reverse(l.code));

    let files = list.iter().map(|l| l.files).sum();
    let code = list.iter().map(|l| l.code).sum();
    let comments = list.iter().map(|l| l.comments).sum();
    let blanks = list.iter().map(|l| l.blanks).sum();
    list.truncate(12);
    (files, code, comments, blanks, list)
}

fn commit_count(repo: &Path) -> u32 {
    run_git(repo, &["rev-list", "--count", "HEAD"])
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn first_commit_date(repo: &Path) -> Option<String> {
    // Root commits (usually one); ISO dates sort lexicographically.
    let raw = run_git(repo, &["log", "--max-parents=0", "--format=%aI", "HEAD"]).ok()?;
    raw.lines().map(str::to_string).min()
}

fn contributors(repo: &Path) -> Vec<Contributor> {
    let Ok(raw) = run_git(repo, &["shortlog", "-sne", "HEAD"]) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            let (count, rest) = line.split_once('\t')?;
            let (name, email) = match rest.rsplit_once(" <") {
                Some((n, e)) => (n.to_string(), e.trim_end_matches('>').to_string()),
                None => (rest.to_string(), String::new()),
            };
            Some(Contributor {
                name,
                email,
                commits: count.trim().parse().ok()?,
            })
        })
        .take(8)
        .collect()
}

/// Everything the Stats tab shows, in one call. Line counting walks the
/// work tree with tokei (respecting .gitignore); the rest comes from git.
pub fn code_stats(repo: &Path) -> Result<CodeStats> {
    let info = super::repo::open_repo(repo)?;
    let root = Path::new(&info.path);
    let (files, code, comments, blanks, languages) = count_lines(root);
    Ok(CodeStats {
        files,
        code,
        comments,
        blanks,
        commits: commit_count(root),
        first_commit_date: first_commit_date(root),
        languages,
        contributors: contributors(root),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;

    #[test]
    fn counts_code_comments_and_git_facts() {
        let repo = TestRepo::with_initial_commit();
        repo.write(
            "src/main.rs",
            "// a comment\nfn main() {\n    println!(\"hi\");\n}\n\n",
        );
        repo.write("web/app.ts", "export const x = 1;\nexport const y = 2;\n");
        repo.git(&["add", "."]);
        repo.commit("add code");

        let s = code_stats(repo.path()).unwrap();
        assert_eq!(s.commits, 2);
        assert!(s.first_commit_date.is_some());
        assert_eq!(s.contributors.len(), 1);
        assert_eq!(s.contributors[0].name, "Test User");
        assert_eq!(s.contributors[0].commits, 2);

        let rust = s.languages.iter().find(|l| l.name == "Rust").unwrap();
        assert_eq!(rust.code, 3);
        assert_eq!(rust.comments, 1);
        assert_eq!(rust.blanks, 1);
        let ts = s.languages.iter().find(|l| l.name == "TypeScript").unwrap();
        assert_eq!(ts.code, 2);
        assert!(s.code >= 5);
        assert!(s.files >= 2);
    }

    #[test]
    fn ignored_directories_are_not_counted() {
        let repo = TestRepo::with_initial_commit();
        repo.write("node_modules/pkg/index.js", "const junk = 1;\n");
        repo.write("real.js", "const real = 1;\n");
        let s = code_stats(repo.path()).unwrap();
        let js = s.languages.iter().find(|l| l.name == "JavaScript").unwrap();
        assert_eq!(js.files, 1);
        assert_eq!(js.code, 1);
    }
}
