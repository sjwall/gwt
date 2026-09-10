use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::repos::get_all_tracked_repos;

/// Error types that can occur during the `list` or `ls` command.
#[derive(Debug)]
pub enum ListError {
    /// Invalid argument count (more than 1 repository specified) (exit code 39).
    InvalidArgCount(String),
    /// Multiple exact repository matches found (exit code 40).
    MultipleExactMatches {
        query: String,
        matches: Vec<PathBuf>,
    },
    /// Multiple repository name matches found (exit code 41).
    MultipleNameMatches {
        query: String,
        matches: Vec<PathBuf>,
    },
    /// Multiple repository path matches found (exit code 42).
    MultiplePathMatches {
        query: String,
        matches: Vec<PathBuf>,
    },
    /// No matching repository found for query (exit code 43).
    NoMatch(String),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl ListError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            ListError::InvalidArgCount(_) => 39,
            ListError::MultipleExactMatches { .. } => 40,
            ListError::MultipleNameMatches { .. } => 41,
            ListError::MultiplePathMatches { .. } => 42,
            ListError::NoMatch(_) => 43,
            ListError::Io(_) => 1,
        }
    }
}

impl fmt::Display for ListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ListError::InvalidArgCount(args) => {
                if args.is_empty() {
                    write!(f, "unknown command: 'list '")
                } else {
                    write!(f, "unknown command: 'list {args}'")
                }
            }
            ListError::MultipleExactMatches { query, matches }
            | ListError::MultipleNameMatches { query, matches }
            | ListError::MultiplePathMatches { query, matches } => {
                write!(f, "multiple repositories match '{query}':")?;
                for m in matches {
                    write!(f, "\n  {}", m.display())?;
                }
                Ok(())
            }
            ListError::NoMatch(query) => {
                write!(f, "no matching repository found for '{query}'")
            }
            ListError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ListError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ListError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for ListError {
    fn from(err: io::Error) -> Self {
        ListError::Io(err)
    }
}

/// CLI arguments for the `list` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct ListArgs {
    /// Arguments and flags for listing worktrees (optional repository name and flags e.g. --porcelain, -v)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl ListArgs {
    /// Creates a new `ListArgs` from a list of arguments.
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    /// Creates a `ListArgs` from an optional repository query string.
    pub fn from_query(query: Option<String>) -> Self {
        Self {
            args: query.into_iter().collect(),
        }
    }

    /// Returns the optional repository query if present.
    pub fn query(&self) -> Option<&str> {
        self.args
            .iter()
            .find(|a| !a.starts_with('-'))
            .map(|s| s.as_str())
    }

    /// Returns the forwarded git flags.
    pub fn flags(&self) -> Vec<&str> {
        self.args
            .iter()
            .filter(|a| a.starts_with('-'))
            .map(|s| s.as_str())
            .collect()
    }
}

/// Separates raw command arguments into git flags (starting with `-`) and positional repository arguments.
pub fn parse_list_args(raw_args: &[String]) -> (Vec<String>, Vec<String>) {
    let mut flags = Vec::new();
    let mut positional = Vec::new();

    for arg in raw_args {
        if arg.starts_with('-') {
            flags.push(arg.clone());
        } else {
            positional.push(arg.clone());
        }
    }

    (flags, positional)
}

/// Resolves which repository or repositories to list worktrees for.
/// If `query` is `None`, returns all tracked repositories.
/// If `query` is `Some`, resolves a single matching repository using exact, name, and path matching.
pub fn resolve_list_repos(
    query: Option<&str>,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<Vec<PathBuf>, ListError> {
    let repos = get_all_tracked_repos(current_dir, config_dir);
    match query {
        None => Ok(repos),
        Some(q) => {
            let query_lower = q.to_lowercase();
            let mut exact_matches = Vec::new();
            let mut matches = Vec::new();
            let mut path_matches = Vec::new();

            for repo in repos {
                let repo_name = repo
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let repo_name_lower = repo_name.to_lowercase();
                let repo_path_lower = repo.to_string_lossy().to_lowercase();

                if repo_name_lower == query_lower {
                    exact_matches.push(repo.clone());
                }
                if repo_name_lower.contains(&query_lower) {
                    matches.push(repo.clone());
                } else if repo_path_lower.contains(&query_lower) {
                    path_matches.push(repo.clone());
                }
            }

            if exact_matches.len() == 1 {
                Ok(vec![exact_matches.into_iter().next().unwrap()])
            } else if exact_matches.len() > 1 {
                Err(ListError::MultipleExactMatches {
                    query: q.to_string(),
                    matches: exact_matches,
                })
            } else if matches.len() == 1 {
                Ok(vec![matches.into_iter().next().unwrap()])
            } else if matches.len() > 1 {
                Err(ListError::MultipleNameMatches {
                    query: q.to_string(),
                    matches,
                })
            } else if path_matches.len() == 1 {
                Ok(vec![path_matches.into_iter().next().unwrap()])
            } else if path_matches.len() > 1 {
                Err(ListError::MultiplePathMatches {
                    query: q.to_string(),
                    matches: path_matches,
                })
            } else {
                Err(ListError::NoMatch(q.to_string()))
            }
        }
    }
}

/// Runs `git worktree list` for a repository with optional flags and returns the output lines.
pub fn get_repo_worktree_lines_with_flags(
    repo: &Path,
    flags: &[String],
) -> io::Result<Vec<String>> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).arg("worktree").arg("list");
    for flag in flags {
        cmd.arg(flag);
    }
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            if stderr.trim().is_empty() {
                "git worktree list command failed".to_string()
            } else {
                stderr.trim().to_string()
            },
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout
        .lines()
        .map(|line| line.to_string())
        .collect();
    Ok(lines)
}

/// Runs `git worktree list` for a repository and returns the output lines.
pub fn get_repo_worktree_lines(repo: &Path) -> io::Result<Vec<String>> {
    get_repo_worktree_lines_with_flags(repo, &[])
}

/// Lists worktrees across tracked repositories (or a single repository if query is provided),
/// forwarding flags to `git worktree list`.
pub fn list_worktrees(
    flags: &[String],
    query: Option<&str>,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<Vec<String>, ListError> {
    let repos = resolve_list_repos(query, current_dir, config_dir)?;
    let mut all_lines = Vec::new();
    for repo in repos {
        let lines = get_repo_worktree_lines_with_flags(&repo, flags)?;
        all_lines.extend(lines);
    }
    Ok(all_lines)
}

/// Executes the `list` / `ls` command with raw argument slice, printing each line to standard output.
pub fn list_cmd(
    raw_args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<Vec<String>, ListError> {
    let (flags, positional) = parse_list_args(raw_args);
    if positional.len() > 1 {
        return Err(ListError::InvalidArgCount(positional.join(" ")));
    }
    let query = positional.first().map(|s| s.as_str());
    let repos = resolve_list_repos(query, current_dir, config_dir)?;
    let mut all_lines = Vec::new();
    for repo in repos {
        let lines = get_repo_worktree_lines_with_flags(&repo, &flags)?;
        for line in &lines {
            println!("{line}");
        }
        all_lines.extend(lines);
    }
    Ok(all_lines)
}

/// Runs the `list` command with parsed `ListArgs`.
pub fn run_args(args: &ListArgs) -> Result<Vec<String>, ListError> {
    list_cmd(&args.args, None, None)
}

/// Runs the `list` command with an optional repository filter.
pub fn run(filter: Option<&str>) -> Result<Vec<String>, ListError> {
    let args: Vec<String> = filter.into_iter().map(String::from).collect();
    list_cmd(&args, None, None)
}

/// Runs the `list` command with raw CLI argument slice.
pub fn run_with_args(args: &[String]) -> Result<Vec<String>, ListError> {
    list_cmd(args, None, None)
}

/// Reads tracked git worktrees across tracked repositories, optionally filtered by repository name.
pub fn list_worktree_lines(
    filter: Option<&str>,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<Vec<String>, ListError> {
    list_worktrees(&[], filter, current_dir, config_dir)
}

/// Prints tracked git worktrees to standard output, matching the shell `gwt ls` output.
pub fn list_and_print(filter: Option<&str>) -> Result<Vec<String>, ListError> {
    run(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn setup_test_git_repo(path: &Path) {
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
    fn test_parse_list_args() {
        let (flags, positional) = parse_list_args(&[]);
        assert!(flags.is_empty());
        assert!(positional.is_empty());

        let raw = vec![
            "--porcelain".to_string(),
            "my-repo".to_string(),
            "-v".to_string(),
        ];
        let (flags, positional) = parse_list_args(&raw);
        assert_eq!(flags, vec!["--porcelain", "-v"]);
        assert_eq!(positional, vec!["my-repo"]);
    }

    #[test]
    fn test_list_args_helpers() {
        let args = ListArgs::new(vec!["--porcelain".to_string(), "my-repo".to_string()]);
        assert_eq!(args.query(), Some("my-repo"));
        assert_eq!(args.flags(), vec!["--porcelain"]);

        let from_q = ListArgs::from_query(Some("repo1".to_string()));
        assert_eq!(from_q.query(), Some("repo1"));
        assert!(from_q.flags().is_empty());

        let empty = ListArgs::default();
        assert_eq!(empty.query(), None);
        assert!(empty.flags().is_empty());
    }

    #[test]
    fn test_list_worktree_lines_all() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_list_all_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        setup_test_git_repo(&repo_dir);

        let lines = list_worktree_lines(None, Some(&repo_dir), Some(&temp_dir)).unwrap();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("main"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_worktrees_with_flags() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_list_flags_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        setup_test_git_repo(&repo_dir);

        // --porcelain outputs worktree, HEAD, branch lines
        let lines = list_worktrees(
            &["--porcelain".to_string()],
            None,
            Some(&repo_dir),
            Some(&temp_dir),
        )
        .unwrap();
        assert!(lines.iter().any(|l| l.starts_with("worktree ")));
        assert!(lines.iter().any(|l| l.starts_with("HEAD ")));
        assert!(lines.iter().any(|l| l.starts_with("branch ")));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_repository_resolution() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_list_res_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo1 = temp_dir.join("alpha-repo");
        let repo2 = temp_dir.join("beta-repo");
        setup_test_git_repo(&repo1);
        setup_test_git_repo(&repo2);

        // Track both repos in config repos file
        let config_dir = temp_dir.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let repos_file = config_dir.join("repos");
        fs::write(
            &repos_file,
            format!("{}\n{}\n", repo1.display(), repo2.display()),
        )
        .unwrap();

        // Query matching alpha
        let alpha_lines =
            list_worktrees(&[], Some("alpha"), None, Some(&config_dir)).unwrap();
        assert_eq!(alpha_lines.len(), 1);
        assert!(alpha_lines[0].contains("alpha-repo"));

        // Query matching beta
        let beta_lines =
            list_worktrees(&[], Some("beta"), None, Some(&config_dir)).unwrap();
        assert_eq!(beta_lines.len(), 1);
        assert!(beta_lines[0].contains("beta-repo"));

        // Query non-existent -> Exit code 43
        let err43 =
            list_worktrees(&[], Some("nonexistent"), None, Some(&config_dir)).unwrap_err();
        assert_eq!(err43.exit_code(), 43);
        assert_eq!(
            err43.to_string(),
            "no matching repository found for 'nonexistent'"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_exit_code_39_invalid_arg_count() {
        let err = list_cmd(&["repo1".to_string(), "repo2".to_string()], None, None)
            .unwrap_err();
        assert_eq!(err.exit_code(), 39);
        assert_eq!(err.to_string(), "unknown command: 'list repo1 repo2'");
    }

    #[test]
    fn test_list_exit_code_40_multiple_exact_matches() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_list_40_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let dir_a = temp_dir.join("a").join("common-name");
        let dir_b = temp_dir.join("b").join("common-name");
        setup_test_git_repo(&dir_a);
        setup_test_git_repo(&dir_b);

        let config_dir = temp_dir.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let repos_file = config_dir.join("repos");
        fs::write(
            &repos_file,
            format!("{}\n{}\n", dir_a.display(), dir_b.display()),
        )
        .unwrap();

        let err =
            list_worktrees(&[], Some("common-name"), None, Some(&config_dir)).unwrap_err();
        assert_eq!(err.exit_code(), 40);
        assert!(err.to_string().contains("multiple repositories match 'common-name':"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_exit_code_41_multiple_name_matches() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_list_41_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let dir_a = temp_dir.join("app-one");
        let dir_b = temp_dir.join("app-two");
        setup_test_git_repo(&dir_a);
        setup_test_git_repo(&dir_b);

        let config_dir = temp_dir.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let repos_file = config_dir.join("repos");
        fs::write(
            &repos_file,
            format!("{}\n{}\n", dir_a.display(), dir_b.display()),
        )
        .unwrap();

        let err = list_worktrees(&[], Some("app"), None, Some(&config_dir)).unwrap_err();
        assert_eq!(err.exit_code(), 41);
        assert!(err.to_string().contains("multiple repositories match 'app':"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_exit_code_42_multiple_path_matches() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_list_42_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let dir_a = temp_dir.join("shared_cluster").join("repo-x");
        let dir_b = temp_dir.join("shared_cluster").join("repo-y");
        setup_test_git_repo(&dir_a);
        setup_test_git_repo(&dir_b);

        let config_dir = temp_dir.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let repos_file = config_dir.join("repos");
        fs::write(
            &repos_file,
            format!("{}\n{}\n", dir_a.display(), dir_b.display()),
        )
        .unwrap();

        let err =
            list_worktrees(&[], Some("shared_cluster"), None, Some(&config_dir)).unwrap_err();
        assert_eq!(err.exit_code(), 42);
        assert!(err.to_string().contains("multiple repositories match 'shared_cluster':"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
