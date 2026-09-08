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
    /// Invalid argument count (more than 1 argument provided) (exit code 37).
    InvalidArgCount(String),
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
            TrackError::InvalidArgCount(_) => 37,
            TrackError::NotAGitRepo(_) => 38,
            TrackError::Io(_) => 1,
        }
    }
}

impl fmt::Display for TrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrackError::NotInsideGitRepo => write!(f, "not inside a git repository"),
            TrackError::InvalidArgCount(args) => write!(f, "unknown command: 'track {args}'"),
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
            if path_str.is_empty() {
                return Err(TrackError::NotAGitRepo(path_str.to_string()));
            }
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

/// Tracks a repository with argument slice.
/// Returns exit code 37 if more than 1 argument is provided.
pub fn track_repo_args(
    args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<PathBuf, TrackError> {
    if args.len() > 1 {
        return Err(TrackError::InvalidArgCount(args.join(" ")));
    }
    let path = args.first().map(|s| s.as_str());
    track_repo(path, current_dir, config_dir)
}

/// Tracks a repository and prints the tracked path to standard output.
pub fn track_and_print(path: Option<&str>) -> Result<PathBuf, TrackError> {
    let repo = track_repo(path, None, None)?;
    println!("{}", repo.display());
    Ok(repo)
}

/// Tracks a repository with arguments and prints the tracked path to standard output.
pub fn track_and_print_args(args: &[String]) -> Result<PathBuf, TrackError> {
    let repo = track_repo_args(args, None, None)?;
    println!("{}", repo.display());
    Ok(repo)
}

/// CLI arguments for the `track` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct TrackArgs {
    /// Path to the git repository to track (defaults to current repository)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl TrackArgs {
    /// Creates a new `TrackArgs` from a list of arguments.
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    /// Creates a `TrackArgs` from an optional path string.
    pub fn from_path(path: Option<String>) -> Self {
        Self {
            args: path.into_iter().collect(),
        }
    }

    /// Returns the optional path if at most one argument was provided.
    pub fn path(&self) -> Option<&str> {
        self.args.first().map(|s| s.as_str())
    }
}

/// Parses CLI arguments for the `track` command.
pub fn parse_track_args(args: &[String]) -> Result<TrackArgs, TrackError> {
    if args.len() > 1 {
        return Err(TrackError::InvalidArgCount(args.join(" ")));
    }
    Ok(TrackArgs {
        args: args.to_vec(),
    })
}

/// Runs the `track` command with parsed `TrackArgs`.
pub fn run_args(args: &TrackArgs) -> Result<PathBuf, TrackError> {
    track_and_print_args(&args.args)
}

/// Runs the `track` command with the provided path.
pub fn run(path: Option<&str>) -> Result<PathBuf, TrackError> {
    track_and_print(path)
}

/// Runs the `track` command with the provided argument slice.
pub fn run_with_args(args: &[String]) -> Result<PathBuf, TrackError> {
    track_and_print_args(args)
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

    #[test]
    fn test_track_error_invalid_arg_count_exit_code_and_display() {
        let err = TrackError::InvalidArgCount("foo bar".to_string());
        assert_eq!(err.exit_code(), 37);
        assert_eq!(err.to_string(), "unknown command: 'track foo bar'");
    }

    #[test]
    fn test_track_multiple_arguments_returns_exit_code_37() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_multi_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let args = vec!["repo1".to_string(), "repo2".to_string()];
        let result = track_repo_args(&args, Some(&repo_dir), Some(&config_dir));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 37);
        assert_eq!(err.to_string(), "unknown command: 'track repo1 repo2'");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_three_arguments_returns_exit_code_37() {
        let args = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = track_repo_args(&args, None, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 37);
        assert_eq!(err.to_string(), "unknown command: 'track a b c'");
    }

    #[test]
    fn test_track_repo_args_empty_tracks_current() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_args_empty_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = track_repo_args(&[], Some(&repo_dir), Some(&config_dir));
        assert!(result.is_ok());

        let tracked_repos = read_tracked_repos(Some(&config_dir)).unwrap();
        assert_eq!(tracked_repos.len(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_repo_args_single_arg_tracks_specified() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_args_single_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let args = vec![repo_dir.to_str().unwrap().to_string()];
        let result = track_repo_args(&args, None, Some(&config_dir));
        assert!(result.is_ok());

        let tracked_repos = read_tracked_repos(Some(&config_dir)).unwrap();
        assert_eq!(tracked_repos.len(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_track_args() {
        let parsed_empty = parse_track_args(&[]).unwrap();
        assert!(parsed_empty.args.is_empty());
        assert_eq!(parsed_empty.path(), None);

        let parsed_single = parse_track_args(&["path/to/repo".to_string()]).unwrap();
        assert_eq!(parsed_single.args.len(), 1);
        assert_eq!(parsed_single.path(), Some("path/to/repo"));

        let err = parse_track_args(&["arg1".to_string(), "arg2".to_string()]).unwrap_err();
        assert_eq!(err.exit_code(), 37);
        assert_eq!(err.to_string(), "unknown command: 'track arg1 arg2'");
    }

    #[test]
    fn test_track_args_constructors() {
        let args = TrackArgs::new(vec!["foo".to_string()]);
        assert_eq!(args.path(), Some("foo"));

        let args_from_path = TrackArgs::from_path(Some("bar".to_string()));
        assert_eq!(args_from_path.path(), Some("bar"));

        let args_from_none = TrackArgs::from_path(None);
        assert_eq!(args_from_none.path(), None);
    }

    #[test]
    fn test_track_empty_string_path_rejected() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_track_empty_str_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = track_repo(Some(""), Some(&repo_dir), Some(&config_dir));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 38);
        assert_eq!(err.to_string(), "not a git repository: ''");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
