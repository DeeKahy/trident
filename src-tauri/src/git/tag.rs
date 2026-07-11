//! Tags and releases. A release in the UI is an annotated tag, optionally
//! pushed to origin.

use std::path::Path;

use super::types::TagInfo;
use super::{run_git, GitError, Result};

/// List tags, newest first. Annotated tags show their own message subject
/// and the commit they point at; lightweight tags fall back to the commit.
pub fn list_tags(repo: &Path) -> Result<Vec<TagInfo>> {
    let format = "%(refname:short)%1f%(objectname:short)%1f%(*objectname:short)%1f%(creatordate:iso-strict)%1f%(subject)";
    let format_arg = format!("--format={format}");
    let raw = run_git(
        repo,
        &[
            "for-each-ref",
            "--sort=-creatordate",
            &format_arg,
            "refs/tags",
        ],
    )?;

    Ok(raw
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\u{1f}');
            let name = fields.next()?.to_string();
            let obj = fields.next()?.to_string();
            let peeled = fields.next()?.to_string();
            let date = fields.next()?.to_string();
            let subject = fields.next().unwrap_or("").to_string();
            Some(TagInfo {
                name,
                // Annotated tags peel to their target commit; lightweight
                // tags point at the commit directly.
                hash: if peeled.is_empty() { obj } else { peeled },
                date,
                subject,
            })
        })
        .collect())
}

/// Create an annotated tag at HEAD (or a plain tag when `message` is
/// empty), optionally pushing it to origin.
pub fn create_tag(repo: &Path, name: &str, message: &str, push: bool) -> Result<()> {
    if name.trim().is_empty() {
        return Err(GitError {
            message: "tag name is empty".to_string(),
            command: String::new(),
            exit_code: None,
        });
    }
    if message.trim().is_empty() {
        run_git(repo, &["tag", "--", name])?;
    } else {
        run_git(repo, &["tag", "-a", "-m", message, "--", name])?;
    }
    if push {
        let refspec = format!("refs/tags/{name}");
        run_git(repo, &["push", "origin", &refspec])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testutil::TestRepo;

    #[test]
    fn create_and_list_annotated_tag() {
        let repo = TestRepo::with_initial_commit();
        create_tag(repo.path(), "v1.0.0", "First release\n\nNotes here.", false).unwrap();

        let tags = list_tags(repo.path()).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0.0");
        assert_eq!(tags[0].subject, "First release");
        let head = repo.git(&["rev-parse", "--short", "HEAD"]);
        assert_eq!(tags[0].hash, head.trim());
    }

    #[test]
    fn lightweight_tag_falls_back_to_commit_subject() {
        let repo = TestRepo::with_initial_commit();
        create_tag(repo.path(), "checkpoint", "", false).unwrap();

        let tags = list_tags(repo.path()).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].subject, "initial commit");
    }

    #[test]
    fn create_tag_can_push_to_origin() {
        let repo = TestRepo::with_initial_commit();
        let bare = repo.add_bare_origin();
        create_tag(repo.path(), "v0.1.0", "push me", true).unwrap();

        let output = std::process::Command::new("git")
            .args(["tag"])
            .current_dir(bare.path())
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&output.stdout).contains("v0.1.0"));
    }

    #[test]
    fn duplicate_tag_errors() {
        let repo = TestRepo::with_initial_commit();
        create_tag(repo.path(), "v1.0.0", "one", false).unwrap();
        assert!(create_tag(repo.path(), "v1.0.0", "two", false).is_err());
    }

    #[test]
    fn empty_name_is_rejected() {
        let repo = TestRepo::with_initial_commit();
        assert!(create_tag(repo.path(), "  ", "msg", false).is_err());
    }
}
