use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::repos::{auto_track_repo, get_current_main_repo, is_git_repo, read_tracked_repos};
use crate::worktree::get_worktrees_for_repo;

/// Error types that can occur during the `cd` command.
#[derive(Debug)]
pub enum CdError {
    /// Invalid argument count (expected exactly 1 argument) (exit code 1).
    InvalidArgCount(String),
    /// Multiple matching worktrees found in current repository (exit code 2).
    MultipleInCurrentRepo {
        query: String,
        matches: Vec<PathBuf>,
    },
    /// Multiple matching worktrees found across tracked repositories (exit code 3).
    MultipleInTrackedRepos {
        query: String,
        matches: Vec<PathBuf>,
    },
    /// No matching worktree found for query (exit code 4).
    NoMatch(String),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl CdError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            CdError::InvalidArgCount(_) => 1,
            CdError::MultipleInCurrentRepo { .. } => 2,
            CdError::MultipleInTrackedRepos { .. } => 3,
            CdError::NoMatch(_) => 4,
            CdError::Io(_) => 1,
        }
    }
}

impl fmt::Display for CdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CdError::InvalidArgCount(args) => {
                if args.is_empty() {
                    write!(f, "unknown command: 'cd '")
                } else {
                    write!(f, "unknown command: 'cd {args}'")
                }
            }
            CdError::MultipleInCurrentRepo { query, matches }
            | CdError::MultipleInTrackedRepos { query, matches } => {
                write!(f, "multiple worktrees match '{query}':")?;
                for m in matches {
                    write!(f, "\n  {}", m.display())?;
                }
                Ok(())
            }
            CdError::NoMatch(query) => {
                write!(f, "no matching worktree found for '{query}'")
            }
            CdError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for CdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CdError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for CdError {
    fn from(err: io::Error) -> Self {
        CdError::Io(err)
    }
}

/// Helper to check if two paths refer to the same repository.
fn is_same_repo(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// Finds exact and substring worktree matches in a list of repository paths.
fn find_worktrees_in_repos(
    repos: &[PathBuf],
    query: &str,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut exact_matches = Vec::new();
    let mut matches = Vec::new();
    let query_lower = query.to_lowercase();

    for repo in repos {
        if !repo.is_dir() {
            continue;
        }
        if let Ok(worktrees) = get_worktrees_for_repo(repo) {
            for wt in worktrees {
                if let Some(name) = wt.name() {
                    let name_lower = name.to_lowercase();
                    if name_lower == query_lower {
                        exact_matches.push(wt.path.clone());
                    }
                    if name_lower.contains(&query_lower) {
                        matches.push(wt.path.clone());
                    }
                }
            }
        }
    }

    (exact_matches, matches)
}

/// Finds the matching worktree path for a query string across current and tracked repositories.
pub fn find_matching_worktree(
    query: &str,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<PathBuf, CdError> {
    let query = query.trim();
    if query.is_empty() {
        return Err(CdError::InvalidArgCount(String::new()));
    }

    let main_repo = get_current_main_repo(current_dir);

    // 1. Check current / main repository if inside a git repo
    if let Some(ref main) = main_repo {
        let _ = auto_track_repo(main, config_dir);
        let (exact_matches, matches) = find_worktrees_in_repos(std::slice::from_ref(main), query);

        if exact_matches.len() == 1 {
            return Ok(exact_matches.into_iter().next().unwrap());
        } else if matches.len() == 1 {
            return Ok(matches.into_iter().next().unwrap());
        } else if matches.len() > 1 {
            return Err(CdError::MultipleInCurrentRepo {
                query: query.to_string(),
                matches,
            });
        }
    }

    // 2. Check other tracked repositories
    let mut other_repos = Vec::new();
    let mut seen = HashSet::new();

    if let Ok(tracked) = read_tracked_repos(config_dir) {
        for r in tracked {
            if !r.is_dir() {
                continue;
            }
            if let Some(ref main) = main_repo {
                if is_same_repo(&r, main) {
                    continue;
                }
            }
            let key = r.to_string_lossy().to_string();
            if !seen.insert(key) {
                continue;
            }
            if !is_git_repo(&r) {
                continue;
            }
            other_repos.push(r);
        }
    }

    if !other_repos.is_empty() {
        let (exact_matches, matches) = find_worktrees_in_repos(&other_repos, query);

        if exact_matches.len() == 1 {
            return Ok(exact_matches.into_iter().next().unwrap());
        } else if matches.len() == 1 {
            return Ok(matches.into_iter().next().unwrap());
        } else if matches.len() > 1 {
            return Err(CdError::MultipleInTrackedRepos {
                query: query.to_string(),
                matches,
            });
        }
    }

    // 3. No match found
    Err(CdError::NoMatch(query.to_string()))
}

/// Executes the `cd` command with the provided argument slice, resolving the matching worktree path.
pub fn cd_worktree(
    args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<PathBuf, CdError> {
    if args.len() != 1 {
        return Err(CdError::InvalidArgCount(args.join(" ")));
    }
    let query = &args[0];
    if query.trim().is_empty() {
        return Err(CdError::InvalidArgCount(String::new()));
    }
    find_matching_worktree(query, current_dir, config_dir)
}

/// Runs the `cd` command and prints the matching worktree path to stdout.
pub fn cd_and_print(args: &[String]) -> Result<PathBuf, CdError> {
    let path = cd_worktree(args, None, None)?;
    println!("{}", path.display());
    Ok(path)
}

/// Runs the `cd` command with CLI arguments.
pub fn run(args: &[String]) -> Result<PathBuf, CdError> {
    cd_and_print(args)
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn add_worktree(repo: &Path, branch: &str, wt_path: &Path) {
        let _ = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["worktree", "add", "-b", branch, wt_path.to_str().unwrap()])
            .output();
    }

    #[test]
    fn test_cd_arg_validation() {
        let err_empty = cd_worktree(&[], None, None).unwrap_err();
        assert_eq!(err_empty.exit_code(), 1);
        assert_eq!(err_empty.to_string(), "unknown command: 'cd '");

        let err_multi = cd_worktree(
            &["feat1".to_string(), "feat2".to_string()],
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(err_multi.exit_code(), 1);
        assert_eq!(err_multi.to_string(), "unknown command: 'cd feat1 feat2'");

        let err_blank = cd_worktree(&["   ".to_string()], None, None).unwrap_err();
        assert_eq!(err_blank.exit_code(), 1);
        assert_eq!(err_blank.to_string(), "unknown command: 'cd '");
    }

    #[test]
    fn test_cd_exact_match_current_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cd_exact_curr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feature-one");
        add_worktree(&repo_dir, "feature-one", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = cd_worktree(
            &["feature-one".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cd_substring_match_current_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cd_sub_curr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feature-xyz");
        add_worktree(&repo_dir, "feature-xyz", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = cd_worktree(&["xyz".to_string()], Some(&repo_dir), Some(&config_dir));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cd_exact_match_preferred_over_substring() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cd_pref_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feat");
        let wt_feat_extra = temp_dir.join("gwt-myrepo").join("feat-extra");
        add_worktree(&repo_dir, "feat", &wt_feat);
        add_worktree(&repo_dir, "feat-extra", &wt_feat_extra);

        let config_dir = temp_dir.join("config");
        let result = cd_worktree(&["feat".to_string()], Some(&repo_dir), Some(&config_dir));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cd_multiple_matches_current_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cd_multi_curr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_fix1 = temp_dir.join("gwt-myrepo").join("fix-bug-1");
        let wt_fix2 = temp_dir.join("gwt-myrepo").join("fix-bug-2");
        add_worktree(&repo_dir, "fix-bug-1", &wt_fix1);
        add_worktree(&repo_dir, "fix-bug-2", &wt_fix2);

        let config_dir = temp_dir.join("config");
        let result = cd_worktree(&["fix".to_string()], Some(&repo_dir), Some(&config_dir));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(err.to_string().contains("multiple worktrees match 'fix':"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cd_match_in_tracked_repos_when_not_in_current() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cd_tracked_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo1 = temp_dir.join("repo1");
        let repo2 = temp_dir.join("repo2");
        init_git_repo(&repo1);
        init_git_repo(&repo2);

        let wt2 = temp_dir.join("gwt-repo2").join("target-feature");
        add_worktree(&repo2, "target-feature", &wt2);

        let config_dir = temp_dir.join("config");
        crate::repos::auto_track_repo(&repo1, Some(&config_dir)).unwrap();
        crate::repos::auto_track_repo(&repo2, Some(&config_dir)).unwrap();

        // Run from repo1 (which does not have target-feature)
        let result = cd_worktree(
            &["target-feature".to_string()],
            Some(&repo1),
            Some(&config_dir),
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt2.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cd_multiple_matches_in_tracked_repos() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cd_multi_tr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo1 = temp_dir.join("repo1");
        let repo2 = temp_dir.join("repo2");
        init_git_repo(&repo1);
        init_git_repo(&repo2);

        let wt1 = temp_dir.join("gwt-repo1").join("hotfix-auth");
        let wt2 = temp_dir.join("gwt-repo2").join("hotfix-payment");
        add_worktree(&repo1, "hotfix-auth", &wt1);
        add_worktree(&repo2, "hotfix-payment", &wt2);

        let config_dir = temp_dir.join("config");
        crate::repos::auto_track_repo(&repo1, Some(&config_dir)).unwrap();
        crate::repos::auto_track_repo(&repo2, Some(&config_dir)).unwrap();

        let plain_dir = temp_dir.join("plain");
        fs::create_dir_all(&plain_dir).unwrap();

        // Run from outside git repositories
        let result = cd_worktree(
            &["hotfix".to_string()],
            Some(&plain_dir),
            Some(&config_dir),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 3);
        assert!(err.to_string().contains("multiple worktrees match 'hotfix':"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cd_no_match() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cd_nomatch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = cd_worktree(
            &["nonexistent".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 4);
        assert_eq!(
            err.to_string(),
            "no matching worktree found for 'nonexistent'"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cd_case_insensitivity() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cd_case_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("my-feature");
        add_worktree(&repo_dir, "my-feature", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = cd_worktree(
            &["MY-FEATURE".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cd_error_exit_codes() {
        assert_eq!(CdError::InvalidArgCount("".into()).exit_code(), 1);
        assert_eq!(
            CdError::MultipleInCurrentRepo {
                query: "q".into(),
                matches: vec![]
            }
            .exit_code(),
            2
        );
        assert_eq!(
            CdError::MultipleInTrackedRepos {
                query: "q".into(),
                matches: vec![]
            }
            .exit_code(),
            3
        );
        assert_eq!(CdError::NoMatch("q".into()).exit_code(), 4);
        assert_eq!(
            CdError::Io(io::Error::new(io::ErrorKind::Other, "io error")).exit_code(),
            1
        );
    }
}
