use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{append_unique_line, expand_tilde, get_app_config_dir, read_lines};

/// Returns the path to the `repos` file in the given config directory (or default).
pub fn get_repos_file(config_dir: Option<&Path>) -> Option<PathBuf> {
    let repos_file = "repos";
    match config_dir {
        Some(p) => Some(p.join(repos_file)),
        None => get_app_config_dir().map(|p| p.join(repos_file)),
    }
}

/// Reads tracked repository paths from the configuration file.
pub fn read_tracked_repos(config_dir: Option<&Path>) -> io::Result<Vec<PathBuf>> {
    let repos_file = match get_repos_file(config_dir) {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    let lines = read_lines(&repos_file)?;
    Ok(lines.into_iter().map(|l| expand_tilde(&l)).collect())
}

/// Automatically tracks a repository path in the `repos` configuration file.
pub fn auto_track_repo(repo_path: &Path, config_dir: Option<&Path>) -> io::Result<bool> {
    let repos_file = match get_repos_file(config_dir) {
        Some(p) => p,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine config directory",
            ));
        }
    };

    let repo_str = repo_path.to_string_lossy().to_string();
    append_unique_line(&repos_file, &repo_str)
}

/// Determines the root/main repository path from the current working directory or given path.
pub fn get_current_main_repo(cwd: Option<&Path>) -> Option<PathBuf> {
    let mut cmd = Command::new("git");
    if let Some(dir) = cwd {
        cmd.arg("-C").arg(dir);
    }
    cmd.args(["worktree", "list", "--porcelain"]);
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(p) = line.strip_prefix("worktree ") {
            let path = PathBuf::from(p.trim());
            if path.is_dir() {
                return Some(path);
            }
        }
    }
    None
}

/// Checks whether a given path is a valid git repository.
pub fn is_git_repo(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--git-dir"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

/// Retrieves all tracked repository paths, including current repository (if applicable) and repos
/// from the configuration file, deduplicating and validating that each exists and is a git repository.
pub fn get_all_tracked_repos(
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut repo_list = Vec::new();
    let main_repo = get_current_main_repo(current_dir);

    if let Some(ref main) = main_repo {
        let _ = auto_track_repo(main, config_dir);
        repo_list.push(main.clone());
    }

    if let Ok(tracked) = read_tracked_repos(config_dir) {
        repo_list.extend(tracked);
    }

    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for repo in repo_list {
        if !repo.is_dir() {
            continue;
        }

        let key = repo.to_string_lossy().to_string();
        if !seen.insert(key) {
            continue;
        }

        if !is_git_repo(&repo) {
            continue;
        }

        result.push(repo);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_tracked_repos() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_repos_read_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let repos_file = temp_dir.join("repos");

        fs::write(&repos_file, "\n/path/repo1\n# comment\n/path/repo2\n").unwrap();

        let repos = read_tracked_repos(Some(&temp_dir)).unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0], PathBuf::from("/path/repo1"));
        assert_eq!(repos[1], PathBuf::from("/path/repo2"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_auto_track_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_repos_track_{}", std::process::id()));
        let repo_path = Path::new("/path/myrepo");

        let added = auto_track_repo(repo_path, Some(&temp_dir)).unwrap();
        assert!(added);

        let added_again = auto_track_repo(repo_path, Some(&temp_dir)).unwrap();
        assert!(!added_again);

        let repos = read_tracked_repos(Some(&temp_dir)).unwrap();
        assert_eq!(repos, vec![PathBuf::from("/path/myrepo")]);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
