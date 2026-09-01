use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::repos::get_all_tracked_repos;

/// Represents a single git worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub path: PathBuf,
    pub commit: String,
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: Option<String>,
    pub prunable: Option<String>,
}

impl Worktree {
    /// Returns the worktree folder name (e.g. `feat-branch`).
    pub fn name(&self) -> Option<&str> {
        self.path.file_name().and_then(|n| n.to_str())
    }
}

/// Parses the output of `git worktree list --porcelain` into a list of [`Worktree`] items.
pub fn parse_porcelain(output: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_head = String::new();
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;
    let mut is_detached = false;
    let mut locked: Option<String> = None;
    let mut prunable: Option<String> = None;

    let flush = |worktrees: &mut Vec<Worktree>,
                 path: &mut Option<PathBuf>,
                 head: &mut String,
                 branch: &mut Option<String>,
                 bare: &mut bool,
                 detached: &mut bool,
                 lock: &mut Option<String>,
                 prune: &mut Option<String>| {
        if let Some(p) = path.take() {
            worktrees.push(Worktree {
                path: p,
                commit: std::mem::take(head),
                branch: branch.take(),
                bare: *bare,
                detached: *detached,
                locked: lock.take(),
                prunable: prune.take(),
            });
            *bare = false;
            *detached = false;
        }
    };

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush(
                &mut worktrees,
                &mut current_path,
                &mut current_head,
                &mut current_branch,
                &mut is_bare,
                &mut is_detached,
                &mut locked,
                &mut prunable,
            );
            continue;
        }

        if let Some(p) = line.strip_prefix("worktree ") {
            flush(
                &mut worktrees,
                &mut current_path,
                &mut current_head,
                &mut current_branch,
                &mut is_bare,
                &mut is_detached,
                &mut locked,
                &mut prunable,
            );
            current_path = Some(PathBuf::from(p));
        } else if let Some(h) = line.strip_prefix("HEAD ") {
            current_head = h.to_string();
        } else if let Some(b) = line.strip_prefix("branch ") {
            let branch_name = b.strip_prefix("refs/heads/").unwrap_or(b);
            current_branch = Some(branch_name.to_string());
        } else if line == "bare" {
            is_bare = true;
        } else if line == "detached" {
            is_detached = true;
        } else if let Some(reason) = line.strip_prefix("locked") {
            let reason = reason.trim();
            locked = Some(if reason.is_empty() {
                String::new()
            } else {
                reason.to_string()
            });
        } else if let Some(reason) = line.strip_prefix("prunable") {
            let reason = reason.trim();
            prunable = Some(if reason.is_empty() {
                String::new()
            } else {
                reason.to_string()
            });
        }
    }

    flush(
        &mut worktrees,
        &mut current_path,
        &mut current_head,
        &mut current_branch,
        &mut is_bare,
        &mut is_detached,
        &mut locked,
        &mut prunable,
    );

    worktrees
}

/// Runs `git worktree list --porcelain` for a repository and returns parsed [`Worktree`] items.
pub fn get_worktrees_for_repo(repo: &Path) -> io::Result<Vec<Worktree>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_porcelain(&stdout))
}

/// Reads all worktrees across all tracked repositories as structured [`Worktree`] records.
pub fn get_all_worktrees(
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> io::Result<Vec<Worktree>> {
    let repos = get_all_tracked_repos(current_dir, config_dir);
    let mut all_worktrees = Vec::new();
    for repo in repos {
        if let Ok(worktrees) = get_worktrees_for_repo(&repo) {
            all_worktrees.extend(worktrees);
        }
    }
    Ok(all_worktrees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain() {
        let sample = "\
worktree /home/user/project
HEAD 0123456789abcdef0123456789abcdef01234567
branch refs/heads/main

worktree /home/user/gwt-project/feat-1
HEAD 1111111111111111111111111111111111111111
branch refs/heads/feat-1

worktree /home/user/gwt-project/detached-wt
HEAD 2222222222222222222222222222222222222222
detached

worktree /home/user/gwt-project/bare-wt
bare
";
        let parsed = parse_porcelain(sample);
        assert_eq!(parsed.len(), 4);

        assert_eq!(parsed[0].path, PathBuf::from("/home/user/project"));
        assert_eq!(parsed[0].commit, "0123456789abcdef0123456789abcdef01234567");
        assert_eq!(parsed[0].branch.as_deref(), Some("main"));
        assert_eq!(parsed[0].name(), Some("project"));
        assert!(!parsed[0].bare);
        assert!(!parsed[0].detached);

        assert_eq!(parsed[1].path, PathBuf::from("/home/user/gwt-project/feat-1"));
        assert_eq!(parsed[1].branch.as_deref(), Some("feat-1"));
        assert_eq!(parsed[1].name(), Some("feat-1"));

        assert_eq!(parsed[2].path, PathBuf::from("/home/user/gwt-project/detached-wt"));
        assert!(parsed[2].detached);
        assert_eq!(parsed[2].branch, None);

        assert_eq!(parsed[3].path, PathBuf::from("/home/user/gwt-project/bare-wt"));
        assert!(parsed[3].bare);
    }

    #[test]
    fn test_parse_porcelain_locked_and_prunable() {
        let sample = "\
worktree /home/user/locked-wt
HEAD 3333333333333333333333333333333333333333
branch refs/heads/locked-branch
locked working on sensitive refactor

worktree /home/user/prunable-wt
HEAD 4444444444444444444444444444444444444444
branch refs/heads/pruned-branch
prunable gitdir path not found
";
        let parsed = parse_porcelain(sample);
        assert_eq!(parsed.len(), 2);

        assert_eq!(parsed[0].path, PathBuf::from("/home/user/locked-wt"));
        assert_eq!(parsed[0].locked.as_deref(), Some("working on sensitive refactor"));
        assert_eq!(parsed[0].prunable, None);

        assert_eq!(parsed[1].path, PathBuf::from("/home/user/prunable-wt"));
        assert_eq!(parsed[1].locked, None);
        assert_eq!(parsed[1].prunable.as_deref(), Some("gitdir path not found"));
    }
}
