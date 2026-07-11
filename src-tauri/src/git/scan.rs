//! Find existing repositories on disk so the hub can add them in bulk.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::types::FoundRepo;

const MAX_DEPTH: usize = 4;

/// Directory names never worth descending into: dependency caches, build
/// output, OS folders. Hidden directories are skipped separately.
const SKIP: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "vendor",
    "venv",
    "Library",
    "Applications",
    "Pictures",
    "Music",
    "Movies",
    "Downloads",
];

/// Folders where developers commonly keep repositories, filtered to the
/// ones that exist on this machine.
pub fn default_roots() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    [
        "Documents", "Desktop", "Developer", "dev", "code", "src", "repos", "git", "Projects",
        "projects", "work", "workspace",
    ]
    .iter()
    .map(|d| home.join(d))
    .filter(|p| p.is_dir())
    .collect()
}

/// Walk `roots` looking for directories that contain `.git`, up to a few
/// levels deep. Paths are canonicalized, so the same repo reached through
/// different routes (or symlinks) appears exactly once.
pub fn scan_for_repos(roots: &[PathBuf]) -> Vec<FoundRepo> {
    let mut found: BTreeMap<PathBuf, FoundRepo> = BTreeMap::new();
    for root in roots {
        walk(root, 0, &mut found);
    }
    found.into_values().collect()
}

fn walk(dir: &Path, depth: usize, found: &mut BTreeMap<PathBuf, FoundRepo>) {
    if depth > MAX_DEPTH {
        return;
    }
    // A .git entry (directory, or file for worktrees) marks a repo root;
    // don't look for nested repos inside it.
    if dir.join(".git").exists() {
        let Ok(canonical) = dir.canonicalize() else {
            return;
        };
        let name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| canonical.to_string_lossy().into_owned());
        found.entry(canonical.clone()).or_insert(FoundRepo {
            path: canonical.to_string_lossy().into_owned(),
            name,
        });
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || SKIP.contains(&name.as_ref()) {
            continue;
        }
        walk(&path, depth + 1, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;

    fn git_init(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn finds_repos_and_skips_junk_and_hidden_dirs() {
        let root = tempfile::tempdir().unwrap();
        git_init(&root.path().join("proj1"));
        git_init(&root.path().join("group/proj2"));
        git_init(&root.path().join("node_modules/somepkg"));
        git_init(&root.path().join(".hidden/secret"));
        std::fs::create_dir_all(root.path().join("plain/folder")).unwrap();

        let found = scan_for_repos(&[root.path().to_path_buf()]);
        let names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"proj1"), "{names:?}");
        assert!(names.contains(&"proj2"), "{names:?}");
        assert_eq!(found.len(), 2, "{names:?}");
    }

    #[test]
    fn does_not_report_nested_repos_or_descend_too_deep() {
        let root = tempfile::tempdir().unwrap();
        git_init(&root.path().join("outer"));
        git_init(&root.path().join("outer/inner")); // inside a repo: ignored
        git_init(&root.path().join("a/b/c/d/e/toodeep"));

        let found = scan_for_repos(&[root.path().to_path_buf()]);
        let names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["outer"], "{names:?}");
    }

    #[test]
    fn same_repo_via_two_roots_appears_once() {
        let repo = TestRepo::with_initial_commit();
        let roots = vec![
            repo.path().parent().unwrap().to_path_buf(),
            repo.path().to_path_buf(),
        ];
        let found = scan_for_repos(&roots);
        let matching: Vec<_> = found
            .iter()
            .filter(|f| f.path == repo.path().canonicalize().unwrap().to_string_lossy())
            .collect();
        assert_eq!(matching.len(), 1);
    }
}
