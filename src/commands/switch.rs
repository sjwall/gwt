use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::commands::cd::{find_matching_worktree, CdError};
use crate::ide::launch_ide;

/// Error types that can occur during the `switch` command.
#[derive(Debug)]
pub enum SwitchError {
    /// Missing required argument for `--ide` option (exit code 11).
    MissingIdeArg,
    /// Invalid argument count (expected exactly 1 worktree name) (exit code 12).
    InvalidArgCount(String),
    /// Error finding or resolving the worktree (exit code 2, 3, 4, etc.).
    Cd(CdError),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl SwitchError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            SwitchError::MissingIdeArg => 11,
            SwitchError::InvalidArgCount(_) => 12,
            SwitchError::Cd(err) => err.exit_code(),
            SwitchError::Io(_) => 1,
        }
    }
}

impl fmt::Display for SwitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwitchError::MissingIdeArg => write!(f, "--ide requires an argument"),
            SwitchError::InvalidArgCount(args) => {
                if args.is_empty() {
                    write!(f, "unknown command: switch ''")
                } else {
                    write!(f, "unknown command: switch '{args}'")
                }
            }
            SwitchError::Cd(err) => write!(f, "{err}"),
            SwitchError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SwitchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SwitchError::Cd(err) => Some(err),
            SwitchError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<CdError> for SwitchError {
    fn from(err: CdError) -> Self {
        SwitchError::Cd(err)
    }
}

impl From<io::Error> for SwitchError {
    fn from(err: io::Error) -> Self {
        SwitchError::Io(err)
    }
}

/// CLI arguments for the `switch` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct SwitchArgs {
    /// Override configured IDE (e.g. nvim, code, cursor, none)
    #[arg(long)]
    pub ide: Option<String>,

    /// Worktree name to match
    pub query: String,
}

pub type SwitchParsedArgs = SwitchArgs;

/// Parses CLI arguments for the `switch` command, supporting `--ide <IDE>` / `--ide=<IDE>`.
pub fn parse_switch_args(args: &[String]) -> Result<SwitchArgs, SwitchError> {
    let mut override_ide = None;
    let mut positional = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if arg == "--ide" {
            if i + 1 >= args.len() {
                return Err(SwitchError::MissingIdeArg);
            }
            override_ide = Some(args[i + 1].clone());
            i += 2;
        } else if let Some(val) = arg.strip_prefix("--ide=") {
            override_ide = Some(val.to_string());
            i += 1;
        } else {
            positional.push(arg.clone());
            i += 1;
        }
    }

    if positional.len() != 1 {
        return Err(SwitchError::InvalidArgCount(positional.join(" ")));
    }

    let query = positional[0].trim().to_string();
    if query.is_empty() {
        return Err(SwitchError::InvalidArgCount(String::new()));
    }

    Ok(SwitchArgs {
        query,
        ide: override_ide,
    })
}

/// Resolves the worktree for the `switch` command with parsed `SwitchArgs` and optionally launches the IDE.
/// Returns the matched worktree path on success.
pub fn switch_worktree_args(
    parsed: &SwitchArgs,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
) -> Result<PathBuf, SwitchError> {
    let path = find_matching_worktree(&parsed.query, current_dir, config_dir)?;

    if launch {
        launch_ide(parsed.ide.as_deref(), &path, config_dir)?;
    }

    Ok(path)
}

/// Resolves the worktree for the `switch` command and optionally launches the IDE.
/// Returns the matched worktree path on success.
pub fn switch_worktree(
    args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
) -> Result<PathBuf, SwitchError> {
    let parsed = parse_switch_args(args)?;
    switch_worktree_args(&parsed, current_dir, config_dir, launch)
}

/// Runs the `switch` command with parsed `SwitchArgs`, printing the matching worktree path to standard output and launching the IDE.
pub fn switch_and_print_args(parsed: &SwitchArgs) -> Result<PathBuf, SwitchError> {
    let path = find_matching_worktree(&parsed.query, None, None)?;
    println!("{}", path.display());
    launch_ide(parsed.ide.as_deref(), &path, None)?;
    Ok(path)
}

/// Runs the `switch` command, printing the matching worktree path to standard output and launching the IDE.
pub fn switch_and_print(args: &[String]) -> Result<PathBuf, SwitchError> {
    let parsed = parse_switch_args(args)?;
    switch_and_print_args(&parsed)
}

/// Runs the `switch` command with parsed `SwitchArgs`.
pub fn run_args(args: &SwitchArgs) -> Result<PathBuf, SwitchError> {
    switch_worktree_args(args, None, None, true)
}

/// Runs the `switch` command with CLI arguments.
pub fn run(args: &[String]) -> Result<PathBuf, SwitchError> {
    switch_worktree(args, None, None, true)
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
    fn test_parse_switch_args() {
        let p1 = parse_switch_args(&["--ide".into(), "code".into(), "my-branch".into()]).unwrap();
        assert_eq!(
            p1,
            SwitchArgs {
                query: "my-branch".into(),
                ide: Some("code".into()),
            }
        );

        let p2 = parse_switch_args(&["--ide=cursor".into(), "feat-x".into()]).unwrap();
        assert_eq!(
            p2,
            SwitchArgs {
                query: "feat-x".into(),
                ide: Some("cursor".into()),
            }
        );

        let p3 = parse_switch_args(&["feat-y".into(), "--ide=none".into()]).unwrap();
        assert_eq!(
            p3,
            SwitchArgs {
                query: "feat-y".into(),
                ide: Some("none".into()),
            }
        );

        let p4 = parse_switch_args(&["feat-z".into()]).unwrap();
        assert_eq!(
            p4,
            SwitchArgs {
                query: "feat-z".into(),
                ide: None,
            }
        );
    }

    #[test]
    fn test_switch_arg_validation() {
        let err_empty = switch_worktree(&[], None, None, false).unwrap_err();
        assert_eq!(err_empty.exit_code(), 12);
        assert_eq!(err_empty.to_string(), "unknown command: switch ''");

        let err_multi = switch_worktree(
            &["feat1".into(), "feat2".into()],
            None,
            None,
            false,
        )
        .unwrap_err();
        assert_eq!(err_multi.exit_code(), 12);
        assert_eq!(err_multi.to_string(), "unknown command: switch 'feat1 feat2'");

        let err_blank = switch_worktree(&["   ".into()], None, None, false).unwrap_err();
        assert_eq!(err_blank.exit_code(), 12);
        assert_eq!(err_blank.to_string(), "unknown command: switch ''");

        let err_missing_ide = switch_worktree(&["--ide".into()], None, None, false).unwrap_err();
        assert_eq!(err_missing_ide.exit_code(), 11);
        assert_eq!(err_missing_ide.to_string(), "--ide requires an argument");

        let err_missing_ide_after_query =
            switch_worktree(&["feat".into(), "--ide".into()], None, None, false).unwrap_err();
        assert_eq!(err_missing_ide_after_query.exit_code(), 11);
        assert_eq!(
            err_missing_ide_after_query.to_string(),
            "--ide requires an argument"
        );
    }

    #[test]
    fn test_switch_exact_match_current_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_exact_curr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feature-one");
        add_worktree(&repo_dir, "feature-one", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = switch_worktree(
            &["feature-one".to_string(), "--ide".to_string(), "none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_switch_substring_match_current_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_sub_curr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feature-xyz");
        add_worktree(&repo_dir, "feature-xyz", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = switch_worktree(
            &["xyz".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_switch_exact_match_preferred_over_substring() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_pref_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feat");
        let wt_feat_extra = temp_dir.join("gwt-myrepo").join("feat-extra");
        add_worktree(&repo_dir, "feat", &wt_feat);
        add_worktree(&repo_dir, "feat-extra", &wt_feat_extra);

        let config_dir = temp_dir.join("config");
        let result = switch_worktree(
            &["feat".to_string(), "--ide".to_string(), "none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_switch_multiple_matches_current_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_multi_curr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_fix1 = temp_dir.join("gwt-myrepo").join("fix-bug-1");
        let wt_fix2 = temp_dir.join("gwt-myrepo").join("fix-bug-2");
        add_worktree(&repo_dir, "fix-bug-1", &wt_fix1);
        add_worktree(&repo_dir, "fix-bug-2", &wt_fix2);

        let config_dir = temp_dir.join("config");
        let result = switch_worktree(
            &["fix".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(err.to_string().contains("multiple worktrees match 'fix':"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_switch_match_in_tracked_repos_when_not_in_current() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_tracked_{}", std::process::id()));
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

        let result = switch_worktree(
            &["target-feature".to_string(), "--ide=none".to_string()],
            Some(&repo1),
            Some(&config_dir),
            true,
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt2.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_switch_multiple_matches_in_tracked_repos() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_multi_tr_{}", std::process::id()));
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

        let result = switch_worktree(
            &["hotfix".to_string(), "--ide=none".to_string()],
            Some(&plain_dir),
            Some(&config_dir),
            true,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 3);
        assert!(err.to_string().contains("multiple worktrees match 'hotfix':"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_switch_no_match() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_nomatch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = switch_worktree(
            &["nonexistent".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
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
    fn test_switch_case_insensitivity() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_case_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("my-feature");
        add_worktree(&repo_dir, "my-feature", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = switch_worktree(
            &["MY-FEATURE".to_string(), "--ide".to_string(), "none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_switch_with_ide_execution() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_switch_ide_exec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feat-ide");
        add_worktree(&repo_dir, "feat-ide", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = switch_worktree(
            &[
                "feat-ide".to_string(),
                "--ide".to_string(),
                "touch switched.txt".to_string(),
            ],
            Some(&repo_dir),
            Some(&config_dir),
            true,
        );
        assert!(result.is_ok());

        let marker_file = wt_feat.join("switched.txt");
        assert!(marker_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_switch_error_exit_codes() {
        assert_eq!(SwitchError::MissingIdeArg.exit_code(), 11);
        assert_eq!(SwitchError::InvalidArgCount("".into()).exit_code(), 12);
        assert_eq!(
            SwitchError::Cd(CdError::MultipleInCurrentRepo {
                query: "q".into(),
                matches: vec![]
            })
            .exit_code(),
            2
        );
        assert_eq!(
            SwitchError::Cd(CdError::MultipleInTrackedRepos {
                query: "q".into(),
                matches: vec![]
            })
            .exit_code(),
            3
        );
        assert_eq!(SwitchError::Cd(CdError::NoMatch("q".into())).exit_code(), 4);
        assert_eq!(
            SwitchError::Io(io::Error::new(io::ErrorKind::Other, "io error")).exit_code(),
            1
        );
    }
}
