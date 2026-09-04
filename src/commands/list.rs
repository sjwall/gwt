use std::io;
use std::path::Path;
use std::process::Command;

use crate::repos::get_all_tracked_repos;

/// Runs `git worktree list` for a repository and returns the output lines.
pub fn get_repo_worktree_lines(repo: &Path) -> io::Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "list"])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect();
    Ok(lines)
}

/// Reads tracked git worktrees across all tracked repositories, matching the output format
/// of the shell `gwt ls` command, optionally filtered by name/query.
pub fn list_worktree_lines(
    filter: Option<&str>,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> io::Result<Vec<String>> {
    let repos = get_all_tracked_repos(current_dir, config_dir);
    let mut all_lines = Vec::new();
    for repo in repos {
        if let Ok(lines) = get_repo_worktree_lines(&repo) {
            for line in lines {
                if let Some(query) = filter {
                    let query_lower = query.to_lowercase();
                    if line.to_lowercase().contains(&query_lower) {
                        all_lines.push(line);
                    }
                } else {
                    all_lines.push(line);
                }
            }
        }
    }
    Ok(all_lines)
}

/// Prints tracked git worktrees to standard output, matching the shell `gwt ls` output.
pub fn list_and_print(filter: Option<&str>) -> io::Result<()> {
    let lines = list_worktree_lines(filter, None, None)?;
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

/// CLI arguments for the `list` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct ListArgs {
    /// Worktree name to match to limit the list to
    pub name: Option<String>,
}

/// Runs the `list` command with parsed `ListArgs`.
pub fn run_args(args: &ListArgs) -> io::Result<()> {
    list_and_print(args.name.as_deref())
}

/// Runs the `list` command with the provided filter.
pub fn run(filter: Option<&str>) -> io::Result<()> {
    list_and_print(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn test_list_worktree_lines() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cmd_list_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        fs::create_dir_all(&repo_dir).unwrap();

        let _ = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["init", "-b", "main"])
            .output();

        let _ = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["config", "user.name", "Test"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["config", "user.email", "test@example.com"])
            .output();

        fs::write(repo_dir.join("README.md"), "hello").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["add", "."])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["commit", "-m", "init"])
            .output();

        let lines = list_worktree_lines(None, Some(&repo_dir), Some(&temp_dir)).unwrap();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("main"));

        let filtered =
            list_worktree_lines(Some("nonexistent"), Some(&repo_dir), Some(&temp_dir)).unwrap();
        assert_eq!(filtered.len(), 0);

        let filtered_match =
            list_worktree_lines(Some("main"), Some(&repo_dir), Some(&temp_dir)).unwrap();
        assert_eq!(filtered_match.len(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
