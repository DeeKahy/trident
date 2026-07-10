//! Test helper: a throwaway git repository in a temp directory.
//!
//! Setup commands run with HOME and git config pointed away from the real
//! user environment, so tests are immune to global hooks, commit signing,
//! and init.defaultBranch settings on the developer's machine.

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

pub struct TestRepo {
    dir: TempDir,
}

impl TestRepo {
    /// A freshly `git init`-ed repo on branch `main`, no commits.
    pub fn empty() -> Self {
        let dir = tempfile::tempdir().expect("create temp dir");
        let repo = Self { dir };
        repo.git(&["init", "--initial-branch=main"]);
        repo.git(&["config", "user.name", "Test User"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    /// A repo with one commit containing `README.md`.
    pub fn with_initial_commit() -> Self {
        let repo = Self::empty();
        repo.write("README.md", "# test repo\n");
        repo.git(&["add", "."]);
        repo.commit("initial commit");
        repo
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Write (or overwrite) a file relative to the repo root.
    pub fn write(&self, rel_path: &str, contents: &str) {
        let full = self.dir.path().join(rel_path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("create parent dirs");
        }
        std::fs::write(full, contents).expect("write file");
    }

    pub fn commit(&self, message: &str) {
        self.git(&["commit", "-m", message]);
    }

    /// Like `git`, but for commands expected to exit non-zero as part of the
    /// fixture setup (e.g. a merge that produces conflicts).
    pub fn git_ignore_status(&self, args: &[&str]) {
        Command::new("git")
            .args(args)
            .current_dir(self.dir.path())
            .env("HOME", self.dir.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("spawn git");
    }

    /// Run a git command in the repo with an isolated environment; panics on
    /// failure since these are test fixtures, not the code under test.
    pub fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.dir.path())
            .env("HOME", self.dir.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_AUTHOR_DATE", "2026-01-02T03:04:05+00:00")
            .env("GIT_COMMITTER_DATE", "2026-01-02T03:04:05+00:00")
            .output()
            .expect("spawn git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}
