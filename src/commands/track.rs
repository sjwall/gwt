use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::config::expand_tilde;
use crate::repos::{auto_track_repo, get_current_main_repo};

/// Error types that can occur during the `track` command.
#[derive(Debug)]
pub enum TrackError {
    /// Not inside a git repository and no repository specified (exit code 36).
    NotInsideGitRepo,
    /// Specified path is not a git repository (exit code 38).
    NotAGitRepo(String),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl TrackError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            TrackError::NotInsideGitRepo => 36,
            TrackError::NotAGitRepo(_) => 38,
            TrackError::Io(_) => 1,
        }
    }
}

impl fmt::Display for TrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrackError::NotInsideGitRepo => write!(f, "not inside a git repository"),
            TrackError::NotAGitRepo(path) => write!(f, "not a git repository: '{path}'"),
            TrackError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for TrackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TrackError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for TrackError {
    fn from(err: io::Error) -> Self {
        TrackError::Io(err)
    }
}

/// Tracks a repository by path (or current directory if `None`), saving it to the `repos` config file.
/// Returns the canonical main repository path on success.
pub fn track_repo(
    path: Option<&str>,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<PathBuf, TrackError> {
    let target_repo = match path {
        None => get_current_main_repo(current_dir).ok_or(TrackError::NotInsideGitRepo)?,
        Some(path_str) => {
            let expanded = expand_tilde(path_str);
            let resolved = if expanded.is_relative() {
                if let Some(cd) = current_dir {
                    cd.join(&expanded)
                } else {
                    expanded
                }
            } else {
                expanded
            };

            if !resolved.is_dir() {
                return Err(TrackError::NotAGitRepo(path_str.to_string()));
            }

            get_current_main_repo(Some(&resolved))
                .ok_or_else(|| TrackError::NotAGitRepo(path_str.to_string()))?
        }
    };

    auto_track_repo(&target_repo, config_dir)?;
    Ok(target_repo)
}

/// Tracks a repository and prints the tracked path to standard output.
pub fn track_and_print(path: Option<&str>) -> Result<PathBuf, TrackError> {
    let repo = track_repo(path, None, None)?;
    println!("{}", repo.display());
    Ok(repo)
}

/// Runs the `track` command with the provided path.
pub fn run(path: Option<&str>) -> Result<PathBuf, TrackError> {
    track_and_print(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::read_tracked_repos;
    use std::fs;
    use std::process::Command;

    fn init_git_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["init", "-b", "main"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["config", "user.name", "Test"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["config", "user.email", "test@example.com"])
            .output();
        fs::write(path.join("README.md"), "hello").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["add", "."])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["commit", "-m", "init"])
            .output();
    }

    #[test]
    fn test_track_current_repo_when_inside_git_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_curr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = track_repo(None, Some(&repo_dir), Some(&config_dir));
        assert!(result.is_ok());

        let tracked_path = result.unwrap();
        let tracked_repos = read_tracked_repos(Some(&config_dir)).unwrap();
        assert_eq!(tracked_repos.len(), 1);
        assert_eq!(tracked_repos[0], tracked_path);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_current_repo_when_not_inside_git_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_not_git_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let plain_dir = temp_dir.join("plain");
        fs::create_dir_all(&plain_dir).unwrap();

        let config_dir = temp_dir.join("config");
        let result = track_repo(None, Some(&plain_dir), Some(&config_dir));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 36);
        assert_eq!(err.to_string(), "not inside a git repository");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_specified_valid_repo_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_spec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = track_repo(
            Some(repo_dir.to_str().unwrap()),
            None,
            Some(&config_dir),
        );
        assert!(result.is_ok());

        let tracked_path = result.unwrap();
        let tracked_repos = read_tracked_repos(Some(&config_dir)).unwrap();
        assert_eq!(tracked_repos.len(), 1);
        assert_eq!(tracked_repos[0], tracked_path);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_specified_relative_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_rel_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = track_repo(Some("myrepo"), Some(&temp_dir), Some(&config_dir));
        assert!(result.is_ok());

        let tracked_path = result.unwrap();
        let tracked_repos = read_tracked_repos(Some(&config_dir)).unwrap();
        assert_eq!(tracked_repos.len(), 1);
        assert_eq!(tracked_repos[0], tracked_path);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_specified_nonexistent_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_nonexistent_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_dir = temp_dir.join("config");
        let result = track_repo(
            Some("/nonexistent/directory/that/does/not/exist"),
            None,
            Some(&config_dir),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 38);
        assert_eq!(
            err.to_string(),
            "not a git repository: '/nonexistent/directory/that/does/not/exist'"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_specified_non_git_dir() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_nongit_spec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let plain_dir = temp_dir.join("plain");
        fs::create_dir_all(&plain_dir).unwrap();

        let config_dir = temp_dir.join("config");
        let result = track_repo(
            Some(plain_dir.to_str().unwrap()),
            None,
            Some(&config_dir),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 38);
        assert!(err.to_string().contains("not a git repository: '"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_idempotent() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_idem_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result1 = track_repo(None, Some(&repo_dir), Some(&config_dir));
        assert!(result1.is_ok());

        let result2 = track_repo(None, Some(&repo_dir), Some(&config_dir));
        assert!(result2.is_ok());

        let tracked_repos = read_tracked_repos(Some(&config_dir)).unwrap();
        assert_eq!(tracked_repos.len(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_from_worktree() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_wt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let main_repo_dir = temp_dir.join("main_repo");
        init_git_repo(&main_repo_dir);

        let wt_dir = temp_dir.join("worktrees").join("feature");
        let _ = Command::new("git")
            .arg("-C")
            .arg(&main_repo_dir)
            .args(["worktree", "add", "-b", "feature", wt_dir.to_str().unwrap()])
            .output();

        let config_dir = temp_dir.join("config");
        let result = track_repo(None, Some(&wt_dir), Some(&config_dir));
        assert!(result.is_ok());

        let tracked_path = result.unwrap();
        // Should point to the main repo, not the linked worktree
        assert_eq!(tracked_path.canonicalize().unwrap_or(tracked_path.clone()), main_repo_dir.canonicalize().unwrap_or(main_repo_dir.clone()));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
