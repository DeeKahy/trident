//! .gitignore edits: the "stop tracking this junk" one-click action.

use std::path::Path;

use super::{GitError, Result};

fn read_gitignore(repo: &Path) -> String {
    std::fs::read_to_string(repo.join(".gitignore")).unwrap_or_default()
}

fn write_gitignore(repo: &Path, contents: &str) -> Result<()> {
    std::fs::write(repo.join(".gitignore"), contents).map_err(|e| GitError {
        message: format!("could not write .gitignore: {e}"),
        command: String::new(),
        exit_code: None,
    })
}

/// Append a path to .gitignore (exact line, created if missing). Adding a
/// line that is already present is a no-op.
pub fn add_to_gitignore(repo: &Path, path: &str) -> Result<()> {
    let line = path.trim();
    if line.is_empty() {
        return Ok(());
    }
    let mut contents = read_gitignore(repo);
    if contents.lines().any(|l| l.trim() == line) {
        return Ok(());
    }
    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(line);
    contents.push('\n');
    write_gitignore(repo, &contents)
}

/// Remove the exact line from .gitignore (the undo of `add_to_gitignore`).
pub fn remove_from_gitignore(repo: &Path, path: &str) -> Result<()> {
    let line = path.trim();
    let contents = read_gitignore(repo);
    let kept: Vec<&str> = contents.lines().filter(|l| l.trim() != line).collect();
    let mut next = kept.join("\n");
    if !next.is_empty() {
        next.push('\n');
    }
    write_gitignore(repo, &next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::status::status;
    use crate::git::testutil::TestRepo;

    #[test]
    fn add_hides_the_file_from_status_and_touches_nothing_else() {
        let repo = TestRepo::with_initial_commit();
        repo.write("junk.log", "x\n");
        repo.write("keep.txt", "y\n");

        add_to_gitignore(repo.path(), "junk.log").unwrap();

        let s = status(repo.path()).unwrap();
        assert!(!s.untracked.contains(&"junk.log".to_string()));
        assert!(s.untracked.contains(&"keep.txt".to_string()));
        assert!(s.untracked.contains(&".gitignore".to_string()));
        // The ignored file itself is untouched on disk.
        assert!(repo.path().join("junk.log").exists());
    }

    #[test]
    fn add_is_idempotent() {
        let repo = TestRepo::with_initial_commit();
        add_to_gitignore(repo.path(), "junk.log").unwrap();
        add_to_gitignore(repo.path(), "junk.log").unwrap();
        let contents = std::fs::read_to_string(repo.path().join(".gitignore")).unwrap();
        assert_eq!(contents.matches("junk.log").count(), 1);
    }

    #[test]
    fn remove_restores_the_file_and_keeps_other_lines() {
        let repo = TestRepo::with_initial_commit();
        repo.write(".gitignore", "node_modules\n");
        repo.git(&["add", ".gitignore"]);
        repo.commit("add gitignore");
        repo.write("junk.log", "x\n");

        add_to_gitignore(repo.path(), "junk.log").unwrap();
        remove_from_gitignore(repo.path(), "junk.log").unwrap();

        let contents = std::fs::read_to_string(repo.path().join(".gitignore")).unwrap();
        assert_eq!(contents, "node_modules\n");
        let s = status(repo.path()).unwrap();
        assert!(s.untracked.contains(&"junk.log".to_string()));
    }
}
