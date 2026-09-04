use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ide::launch_ide;
use crate::locations::determine_dir_gwt;
use crate::repos::{auto_track_repo, get_current_main_repo};

/// Error types that can occur during the `pull` command.
#[derive(Debug)]
pub enum PullError {
    /// Missing required argument for `--ide` option (exit code 16).
    MissingIdeArg,
    /// Invalid argument count (expected exactly 1 branch name) (exit code 17).
    InvalidArgCount(String),
    /// Failed to determine target worktree directory location (exit code 18).
    DetermineTargetLocation,
    /// Failed to create worktree parent directory (exit code 19).
    CreateParentDir(String),
    /// `git fetch origin` command failed (exit code 20).
    GitFetchOrigin(String),
    /// `git worktree add` command failed (exit code 21).
    GitWorktreeAdd(String),
    /// Failed to change directory to newly created worktree (exit code 22).
    CdWorktree(String),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl PullError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            PullError::MissingIdeArg => 16,
            PullError::InvalidArgCount(_) => 17,
            PullError::DetermineTargetLocation => 18,
            PullError::CreateParentDir(_) => 19,
            PullError::GitFetchOrigin(_) => 20,
            PullError::GitWorktreeAdd(_) => 21,
            PullError::CdWorktree(_) => 22,
            PullError::Io(_) => 1,
        }
    }
}

impl fmt::Display for PullError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PullError::MissingIdeArg => write!(f, "--ide requires an argument"),
            PullError::InvalidArgCount(args) => {
                if args.is_empty() {
                    write!(f, "unknown command: 'pull '")
                } else {
                    write!(f, "unknown command: 'pull {args}'")
                }
            }
            PullError::DetermineTargetLocation => {
                write!(f, "failed to determine target worktree directory location")
            }
            PullError::CreateParentDir(msg) => {
                if msg.is_empty() {
                    write!(f, "failed to create worktree parent directory")
                } else {
                    write!(f, "failed to create worktree parent directory: {msg}")
                }
            }
            PullError::GitFetchOrigin(msg) => {
                if msg.is_empty() {
                    write!(f, "git fetch origin command failed")
                } else {
                    write!(f, "git fetch origin command failed: {msg}")
                }
            }
            PullError::GitWorktreeAdd(msg) => {
                if msg.is_empty() {
                    write!(f, "git worktree add command failed")
                } else {
                    write!(f, "git worktree add command failed: {msg}")
                }
            }
            PullError::CdWorktree(msg) => {
                if msg.is_empty() {
                    write!(f, "failed to change directory to newly created worktree")
                } else {
                    write!(f, "failed to change directory to newly created worktree: {msg}")
                }
            }
            PullError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for PullError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PullError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for PullError {
    fn from(err: io::Error) -> Self {
        PullError::Io(err)
    }
}

/// CLI arguments for the `pull` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct PullArgs {
    /// Override configured IDE (e.g. nvim, code, cursor, none)
    #[arg(long)]
    pub ide: Option<String>,

    /// Skip running yarn install
    #[arg(long)]
    pub no_install: bool,

    /// Remote branch name to fetch and create worktree for
    pub branch: String,
}

pub type PullParsedArgs = PullArgs;

/// Parses CLI arguments for the `pull` command, supporting `--ide <IDE>`, `--ide=<IDE>`, and `--no-install`.
pub fn parse_pull_args(args: &[String]) -> Result<PullArgs, PullError> {
    let mut override_ide = None;
    let mut no_install = false;
    let mut positional = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if arg == "--ide" {
            if i + 1 >= args.len() {
                return Err(PullError::MissingIdeArg);
            }
            override_ide = Some(args[i + 1].clone());
            i += 2;
        } else if let Some(val) = arg.strip_prefix("--ide=") {
            override_ide = Some(val.to_string());
            i += 1;
        } else if arg == "--no-install" {
            no_install = true;
            i += 1;
        } else {
            positional.push(arg.clone());
            i += 1;
        }
    }

    if positional.len() != 1 {
        return Err(PullError::InvalidArgCount(positional.join(" ")));
    }

    let branch = positional[0].trim().to_string();
    if branch.is_empty() {
        return Err(PullError::InvalidArgCount(String::new()));
    }

    Ok(PullArgs {
        branch,
        ide: override_ide,
        no_install,
    })
}

/// Determines the worktree parent directory (`dir_gwt`) for a repository.
pub fn get_dir_gwt<R: io::BufRead>(
    target_repo: &Path,
    config_dir: Option<&Path>,
    prompt_reader: Option<&mut R>,
) -> Result<PathBuf, PullError> {
    determine_dir_gwt(target_repo, config_dir, prompt_reader)
        .ok_or(PullError::DetermineTargetLocation)
}

/// Fetches `origin/<name>`, creates a tracking worktree for the specified branch name,
/// runs `yarn` if `yarn.lock` exists, and launches the configured IDE unless disabled.
pub fn pull_worktree_args<R: io::BufRead>(
    parsed: &PullArgs,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
    prompt_reader: Option<&mut R>,
) -> Result<PathBuf, PullError> {
    let main_repo = get_current_main_repo(current_dir);
    if let Some(ref main) = main_repo {
        let _ = auto_track_repo(main, config_dir);
    }

    let default_cwd;
    let target_repo = match main_repo.as_deref() {
        Some(main) => main,
        None => match current_dir {
            Some(cd) => cd,
            None => {
                default_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                &default_cwd
            }
        },
    };

    let dir_gwt = get_dir_gwt(target_repo, config_dir, prompt_reader)?;

    if let Err(err) = std::fs::create_dir_all(&dir_gwt) {
        return Err(PullError::CreateParentDir(err.to_string()));
    }

    let mut fetch_cmd = Command::new("git");
    fetch_cmd.arg("-C").arg(target_repo);
    fetch_cmd.args(["fetch", "origin", &parsed.branch]);

    let fetch_status = fetch_cmd
        .status()
        .map_err(|e| PullError::GitFetchOrigin(e.to_string()))?;
    if !fetch_status.success() {
        return Err(PullError::GitFetchOrigin(String::new()));
    }

    let dest = dir_gwt.join(&parsed.branch);

    let mut git_cmd = Command::new("git");
    git_cmd.arg("-C").arg(target_repo);
    git_cmd.args([
        "worktree",
        "add",
        "-b",
        &parsed.branch,
        dest.to_str().unwrap(),
        &format!("origin/{}", parsed.branch),
    ]);

    let status = git_cmd
        .status()
        .map_err(|e| PullError::GitWorktreeAdd(e.to_string()))?;
    if !status.success() {
        return Err(PullError::GitWorktreeAdd(String::new()));
    }

    if !dest.is_dir() {
        return Err(PullError::CdWorktree(String::new()));
    }

    if !parsed.no_install && dest.join("yarn.lock").is_file() {
        let _ = Command::new("yarn")
            .current_dir(&dest)
            .status();
    }

    if launch {
        launch_ide(parsed.ide.as_deref(), &dest, config_dir)?;
    }

    Ok(dest)
}

/// Fetches `origin/<name>`, creates a tracking worktree for the specified branch name,
/// runs `yarn` if `yarn.lock` exists, and launches the configured IDE unless disabled.
pub fn pull_worktree<R: io::BufRead>(
    args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
    prompt_reader: Option<&mut R>,
) -> Result<PathBuf, PullError> {
    let parsed = parse_pull_args(args)?;
    pull_worktree_args(&parsed, current_dir, config_dir, launch, prompt_reader)
}

/// Runs the `pull` command with parsed `PullArgs`.
pub fn run_args(args: &PullArgs) -> Result<PathBuf, PullError> {
    pull_worktree_args(args, None, None, true, None::<&mut io::Empty>)
}

/// Runs the `pull` command with CLI arguments.
pub fn run(args: &[String]) -> Result<PathBuf, PullError> {
    pull_worktree(args, None, None, true, None::<&mut io::Empty>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::process::Command;

    use crate::locations::save_configured_parent;

    fn init_git_repo_with_origin(local_path: &Path, origin_path: &Path, remote_branch: &str) {
        fs::create_dir_all(origin_path).unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(origin_path)
            .args(["init", "--bare"])
            .output();

        fs::create_dir_all(local_path).unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["init", "-b", "main"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["config", "user.name", "Test"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["config", "user.email", "test@example.com"])
            .output();
        fs::write(local_path.join("README.md"), "hello").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["add", "."])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["commit", "-m", "init"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["remote", "add", "origin", origin_path.to_str().unwrap()])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["push", "-u", "origin", "main"])
            .output();

        // Create a remote branch on origin
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["checkout", "-b", remote_branch])
            .output();
        fs::write(local_path.join("remote_file.txt"), "feature").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["add", "."])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["commit", "-m", "feat commit"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["push", "origin", remote_branch])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["checkout", "main"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(local_path)
            .args(["branch", "-D", remote_branch])
            .output();
    }

    #[test]
    fn test_parse_pull_args() {
        let p1 = parse_pull_args(&["my-branch".into()]).unwrap();
        assert_eq!(
            p1,
            PullArgs {
                branch: "my-branch".into(),
                ide: None,
                no_install: false,
            }
        );

        let p2 = parse_pull_args(&["--ide".into(), "code".into(), "feat-x".into()]).unwrap();
        assert_eq!(
            p2,
            PullArgs {
                branch: "feat-x".into(),
                ide: Some("code".into()),
                no_install: false,
            }
        );

        let p3 = parse_pull_args(&["--ide=cursor".into(), "--no-install".into(), "feat-y".into()]).unwrap();
        assert_eq!(
            p3,
            PullArgs {
                branch: "feat-y".into(),
                ide: Some("cursor".into()),
                no_install: true,
            }
        );

        let p4 = parse_pull_args(&["feat-z".into(), "--no-install".into(), "--ide".into(), "none".into()]).unwrap();
        assert_eq!(
            p4,
            PullArgs {
                branch: "feat-z".into(),
                ide: Some("none".into()),
                no_install: true,
            }
        );
    }

    #[test]
    fn test_parse_pull_arg_validation() {
        let err_empty = parse_pull_args(&[]).unwrap_err();
        assert_eq!(err_empty.exit_code(), 17);
        assert_eq!(err_empty.to_string(), "unknown command: 'pull '");

        let err_blank = parse_pull_args(&["   ".into()]).unwrap_err();
        assert_eq!(err_blank.exit_code(), 17);
        assert_eq!(err_blank.to_string(), "unknown command: 'pull '");

        let err_multi = parse_pull_args(&["feat1".into(), "feat2".into()]).unwrap_err();
        assert_eq!(err_multi.exit_code(), 17);
        assert_eq!(err_multi.to_string(), "unknown command: 'pull feat1 feat2'");

        let err_missing_ide = parse_pull_args(&["--ide".into()]).unwrap_err();
        assert_eq!(err_missing_ide.exit_code(), 16);
        assert_eq!(err_missing_ide.to_string(), "--ide requires an argument");

        let err_missing_ide_after_branch = parse_pull_args(&["feat".into(), "--ide".into()]).unwrap_err();
        assert_eq!(err_missing_ide_after_branch.exit_code(), 16);
        assert_eq!(err_missing_ide_after_branch.to_string(), "--ide requires an argument");
    }

    #[test]
    fn test_pull_worktree_basic() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_pull_basic_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let origin_dir = temp_dir.join("origin.git");
        let repo_dir = temp_dir.join("myrepo");
        init_git_repo_with_origin(&repo_dir, &origin_dir, "remote-feat");

        let config_dir = temp_dir.join("config");
        let result = pull_worktree(
            &["remote-feat".to_string(), "--ide".to_string(), "none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            None::<&mut Cursor<Vec<u8>>>,
        );

        assert!(result.is_ok());
        let dest = result.unwrap();
        assert!(dest.is_dir());
        assert_eq!(
            dest.canonicalize().unwrap(),
            temp_dir.join("gwt-myrepo").join("remote-feat").canonicalize().unwrap()
        );
        assert!(dest.join("remote_file.txt").exists());

        // Verify git recognizes the new worktree
        let wt_list = crate::worktree::get_worktrees_for_repo(&repo_dir).unwrap();
        assert_eq!(wt_list.len(), 2);
        assert!(wt_list.iter().any(|wt| wt.branch.as_deref() == Some("remote-feat")));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pull_worktree_with_configured_parent() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_pull_cfg_parent_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let origin_dir = temp_dir.join("origin.git");
        let repo_dir = temp_dir.join("repo");
        init_git_repo_with_origin(&repo_dir, &origin_dir, "feature-custom");

        let custom_parent = temp_dir.join("custom_parent");
        fs::create_dir_all(&custom_parent).unwrap();

        let config_dir = temp_dir.join("config");
        save_configured_parent(&repo_dir, &custom_parent, Some(&config_dir)).unwrap();

        let result = pull_worktree(
            &["feature-custom".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            None::<&mut Cursor<Vec<u8>>>,
        );

        assert!(result.is_ok());
        let dest = result.unwrap();
        assert_eq!(
            dest.canonicalize().unwrap(),
            custom_parent.join("gwt-repo").join("feature-custom").canonicalize().unwrap()
        );
        assert!(dest.is_dir());
        assert!(dest.join("remote_file.txt").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pull_worktree_unsuitable_path_prompt() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_pull_unsuit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let hidden_dir = temp_dir.join(".hidden");
        fs::create_dir_all(&hidden_dir).unwrap();

        let origin_dir = temp_dir.join("origin.git");
        let repo_dir = hidden_dir.join("myrepo");
        init_git_repo_with_origin(&repo_dir, &origin_dir, "feat-prompt");

        let safe_parent = temp_dir.join("safe_worktrees");
        fs::create_dir_all(&safe_parent).unwrap();

        let config_dir = temp_dir.join("config");
        let mut prompt_input = Cursor::new(format!("{}\n", safe_parent.display()).into_bytes());

        let result = pull_worktree(
            &["feat-prompt".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            Some(&mut prompt_input),
        );

        assert!(result.is_ok());
        let dest = result.unwrap();
        assert_eq!(
            dest.canonicalize().unwrap(),
            safe_parent.join("gwt-myrepo").join("feat-prompt").canonicalize().unwrap()
        );
        assert!(dest.is_dir());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pull_worktree_unsuitable_path_empty_prompt_fails() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_pull_unsuit_fail_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let hidden_dir = temp_dir.join(".hidden");
        fs::create_dir_all(&hidden_dir).unwrap();

        let origin_dir = temp_dir.join("origin.git");
        let repo_dir = hidden_dir.join("myrepo");
        init_git_repo_with_origin(&repo_dir, &origin_dir, "feat-prompt-fail");

        let config_dir = temp_dir.join("config");
        let mut empty_input = Cursor::new(b"\n".to_vec());

        let result = pull_worktree(
            &["feat-prompt-fail".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            Some(&mut empty_input),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 18);
        assert_eq!(
            err.to_string(),
            "failed to determine target worktree directory location"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pull_worktree_fetch_nonexistent_branch_fails() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_pull_fail_fetch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let origin_dir = temp_dir.join("origin.git");
        let repo_dir = temp_dir.join("myrepo");
        init_git_repo_with_origin(&repo_dir, &origin_dir, "existing-feat");

        let config_dir = temp_dir.join("config");
        let result = pull_worktree(
            &["nonexistent-branch".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            None::<&mut Cursor<Vec<u8>>>,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 20);
        assert!(err.to_string().contains("git fetch origin command failed"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pull_worktree_add_fails_when_branch_already_checked_out() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_pull_fail_add_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let origin_dir = temp_dir.join("origin.git");
        let repo_dir = temp_dir.join("myrepo");
        init_git_repo_with_origin(&repo_dir, &origin_dir, "same-branch");

        let config_dir = temp_dir.join("config");

        // First pull succeeds
        let res1 = pull_worktree(
            &["same-branch".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            None::<&mut Cursor<Vec<u8>>>,
        );
        assert!(res1.is_ok());

        // Second pull fails at worktree add because branch already checked out
        let res2 = pull_worktree(
            &["same-branch".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            None::<&mut Cursor<Vec<u8>>>,
        );
        assert!(res2.is_err());
        let err = res2.unwrap_err();
        assert_eq!(err.exit_code(), 21);
        assert!(err.to_string().contains("git worktree add command failed"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pull_worktree_with_ide_execution() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_pull_ide_exec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let origin_dir = temp_dir.join("origin.git");
        let repo_dir = temp_dir.join("myrepo");
        init_git_repo_with_origin(&repo_dir, &origin_dir, "feat-ide");

        let config_dir = temp_dir.join("config");
        let result = pull_worktree(
            &[
                "feat-ide".to_string(),
                "--ide".to_string(),
                "touch created_by_pull.txt".to_string(),
            ],
            Some(&repo_dir),
            Some(&config_dir),
            true,
            None::<&mut Cursor<Vec<u8>>>,
        );
        assert!(result.is_ok());

        let wt_path = temp_dir.join("gwt-myrepo").join("feat-ide");
        let marker_file = wt_path.join("created_by_pull.txt");
        assert!(marker_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pull_error_exit_codes() {
        assert_eq!(PullError::MissingIdeArg.exit_code(), 16);
        assert_eq!(PullError::InvalidArgCount("".into()).exit_code(), 17);
        assert_eq!(PullError::DetermineTargetLocation.exit_code(), 18);
        assert_eq!(PullError::CreateParentDir("".into()).exit_code(), 19);
        assert_eq!(PullError::GitFetchOrigin("".into()).exit_code(), 20);
        assert_eq!(PullError::GitWorktreeAdd("".into()).exit_code(), 21);
        assert_eq!(PullError::CdWorktree("".into()).exit_code(), 22);
        assert_eq!(
            PullError::Io(io::Error::other("io err")).exit_code(),
            1
        );
    }
}
