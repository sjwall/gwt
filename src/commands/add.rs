use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ide::launch_ide;
use crate::locations::determine_dir_gwt;
use crate::repos::{auto_track_repo, get_current_main_repo};

/// Error types that can occur during the `add` command.
#[derive(Debug)]
pub enum AddError {
    /// Missing required argument for `--ide` option (exit code 23).
    MissingIdeArg,
    /// Invalid argument count (expected exactly 1 branch name) (exit code 24).
    InvalidArgCount(String),
    /// Failed to determine target worktree directory location (exit code 25).
    DetermineTargetLocation,
    /// Failed to create worktree parent directory (exit code 26).
    CreateParentDir(String),
    /// `git worktree add` command failed (exit code 27).
    GitWorktreeAdd(String),
    /// Failed to change directory to newly created worktree (exit code 28).
    CdWorktree(String),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl AddError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            AddError::MissingIdeArg => 23,
            AddError::InvalidArgCount(_) => 24,
            AddError::DetermineTargetLocation => 25,
            AddError::CreateParentDir(_) => 26,
            AddError::GitWorktreeAdd(_) => 27,
            AddError::CdWorktree(_) => 28,
            AddError::Io(_) => 1,
        }
    }
}

impl fmt::Display for AddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddError::MissingIdeArg => write!(f, "--ide requires an argument"),
            AddError::InvalidArgCount(args) => {
                if args.is_empty() {
                    write!(f, "unknown command: ''")
                } else {
                    write!(f, "unknown command: '{args}'")
                }
            }
            AddError::DetermineTargetLocation => {
                write!(f, "failed to determine target worktree directory location")
            }
            AddError::CreateParentDir(msg) => {
                if msg.is_empty() {
                    write!(f, "failed to create worktree parent directory")
                } else {
                    write!(f, "failed to create worktree parent directory: {msg}")
                }
            }
            AddError::GitWorktreeAdd(msg) => {
                if msg.is_empty() {
                    write!(f, "git worktree add command failed")
                } else {
                    write!(f, "git worktree add command failed: {msg}")
                }
            }
            AddError::CdWorktree(msg) => {
                if msg.is_empty() {
                    write!(f, "failed to change directory to newly created worktree")
                } else {
                    write!(f, "failed to change directory to newly created worktree: {msg}")
                }
            }
            AddError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for AddError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AddError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AddError {
    fn from(err: io::Error) -> Self {
        AddError::Io(err)
    }
}

/// CLI arguments for the `add` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct AddArgs {
    /// Override configured IDE (e.g. nvim, code, cursor, none)
    #[arg(long)]
    pub ide: Option<String>,

    /// Skip running yarn install
    #[arg(long)]
    pub no_install: bool,

    /// Branch name for the new worktree
    pub branch: String,
}

pub type AddParsedArgs = AddArgs;

/// Parses CLI arguments for the `add` command, supporting `--ide <IDE>`, `--ide=<IDE>`, and `--no-install`.
pub fn parse_add_args(args: &[String]) -> Result<AddArgs, AddError> {
    let mut override_ide = None;
    let mut no_install = false;
    let mut positional = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if arg == "--ide" {
            if i + 1 >= args.len() {
                return Err(AddError::MissingIdeArg);
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
        return Err(AddError::InvalidArgCount(positional.join(" ")));
    }

    let branch = positional[0].trim().to_string();
    if branch.is_empty() {
        return Err(AddError::InvalidArgCount(String::new()));
    }

    Ok(AddArgs {
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
) -> Result<PathBuf, AddError> {
    determine_dir_gwt(target_repo, config_dir, prompt_reader)
        .ok_or(AddError::DetermineTargetLocation)
}

/// Creates a new worktree for the specified branch name, running `yarn` if `yarn.lock` exists,
/// and launching the configured IDE unless disabled.
pub fn add_worktree_args<R: io::BufRead>(
    parsed: &AddArgs,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
    prompt_reader: Option<&mut R>,
) -> Result<PathBuf, AddError> {
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
        return Err(AddError::CreateParentDir(err.to_string()));
    }

    let dest = dir_gwt.join(&parsed.branch);

    let mut git_cmd = Command::new("git");
    git_cmd.arg("-C").arg(target_repo);
    git_cmd.args(["worktree", "add", dest.to_str().unwrap()]);

    let status = git_cmd.status().map_err(|e| AddError::GitWorktreeAdd(e.to_string()))?;
    if !status.success() {
        return Err(AddError::GitWorktreeAdd(String::new()));
    }

    if !dest.is_dir() {
        return Err(AddError::CdWorktree(String::new()));
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

/// Creates a new worktree for the specified branch name, running `yarn` if `yarn.lock` exists,
/// and launching the configured IDE unless disabled.
pub fn add_worktree<R: io::BufRead>(
    args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
    prompt_reader: Option<&mut R>,
) -> Result<PathBuf, AddError> {
    let parsed = parse_add_args(args)?;
    add_worktree_args(&parsed, current_dir, config_dir, launch, prompt_reader)
}

/// Runs the `add` command with parsed `AddArgs`.
pub fn run_args(args: &AddArgs) -> Result<PathBuf, AddError> {
    add_worktree_args(args, None, None, true, None::<&mut io::Empty>)
}

/// Runs the `add` command with CLI arguments.
pub fn run(args: &[String]) -> Result<PathBuf, AddError> {
    add_worktree(args, None, None, true, None::<&mut io::Empty>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::process::Command;

    use crate::locations::{get_configured_parent, save_configured_parent};

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
    fn test_parse_add_args() {
        let p1 = parse_add_args(&["my-branch".into()]).unwrap();
        assert_eq!(
            p1,
            AddArgs {
                branch: "my-branch".into(),
                ide: None,
                no_install: false,
            }
        );

        let p2 = parse_add_args(&["--ide".into(), "code".into(), "feat-x".into()]).unwrap();
        assert_eq!(
            p2,
            AddArgs {
                branch: "feat-x".into(),
                ide: Some("code".into()),
                no_install: false,
            }
        );

        let p3 = parse_add_args(&["--ide=cursor".into(), "--no-install".into(), "feat-y".into()]).unwrap();
        assert_eq!(
            p3,
            AddArgs {
                branch: "feat-y".into(),
                ide: Some("cursor".into()),
                no_install: true,
            }
        );

        let p4 = parse_add_args(&["feat-z".into(), "--no-install".into(), "--ide".into(), "none".into()]).unwrap();
        assert_eq!(
            p4,
            AddArgs {
                branch: "feat-z".into(),
                ide: Some("none".into()),
                no_install: true,
            }
        );
    }

    #[test]
    fn test_parse_add_arg_validation() {
        let err_empty = parse_add_args(&[]).unwrap_err();
        assert_eq!(err_empty.exit_code(), 24);
        assert_eq!(err_empty.to_string(), "unknown command: ''");

        let err_blank = parse_add_args(&["   ".into()]).unwrap_err();
        assert_eq!(err_blank.exit_code(), 24);
        assert_eq!(err_blank.to_string(), "unknown command: ''");

        let err_multi = parse_add_args(&["feat1".into(), "feat2".into()]).unwrap_err();
        assert_eq!(err_multi.exit_code(), 24);
        assert_eq!(err_multi.to_string(), "unknown command: 'feat1 feat2'");

        let err_missing_ide = parse_add_args(&["--ide".into()]).unwrap_err();
        assert_eq!(err_missing_ide.exit_code(), 23);
        assert_eq!(err_missing_ide.to_string(), "--ide requires an argument");

        let err_missing_ide_after_branch = parse_add_args(&["feat".into(), "--ide".into()]).unwrap_err();
        assert_eq!(err_missing_ide_after_branch.exit_code(), 23);
        assert_eq!(err_missing_ide_after_branch.to_string(), "--ide requires an argument");
    }

    #[test]
    fn test_add_worktree_basic() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_add_basic_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = add_worktree(
            &["feature-1".to_string(), "--ide".to_string(), "none".to_string()],
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
            temp_dir.join("gwt-myrepo").join("feature-1").canonicalize().unwrap()
        );

        // Verify git recognizes the new worktree
        let wt_list = crate::worktree::get_worktrees_for_repo(&repo_dir).unwrap();
        assert_eq!(wt_list.len(), 2);
        assert!(wt_list.iter().any(|wt| wt.branch.as_deref() == Some("feature-1")));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_add_worktree_with_configured_parent() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_add_cfg_parent_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let custom_parent = temp_dir.join("custom_parent");
        fs::create_dir_all(&custom_parent).unwrap();

        let config_dir = temp_dir.join("config");
        save_configured_parent(&repo_dir, &custom_parent, Some(&config_dir)).unwrap();

        let result = add_worktree(
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

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_add_worktree_unsuitable_path_prompt() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_add_unsuit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let hidden_dir = temp_dir.join(".hidden");
        fs::create_dir_all(&hidden_dir).unwrap();

        let repo_dir = hidden_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let safe_parent = temp_dir.join("safe_worktrees");
        fs::create_dir_all(&safe_parent).unwrap();

        let config_dir = temp_dir.join("config");
        let mut prompt_input = Cursor::new(format!("{}\n", safe_parent.display()).into_bytes());

        let result = add_worktree(
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

        // Verify it was saved to locations
        let saved = get_configured_parent(&repo_dir, Some(&config_dir));
        assert!(saved.is_some());
        assert_eq!(
            saved.unwrap().canonicalize().unwrap(),
            safe_parent.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_add_worktree_unsuitable_path_empty_prompt_fails() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_add_unsuit_fail_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let hidden_dir = temp_dir.join(".hidden");
        fs::create_dir_all(&hidden_dir).unwrap();

        let repo_dir = hidden_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let mut empty_input = Cursor::new(b"\n".to_vec());

        let result = add_worktree(
            &["feat-prompt-fail".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            Some(&mut empty_input),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 25);
        assert_eq!(
            err.to_string(),
            "failed to determine target worktree directory location"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_add_worktree_failure_branch_exists() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_add_fail_branch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");

        // First add succeeds
        let res1 = add_worktree(
            &["same-branch".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            None::<&mut Cursor<Vec<u8>>>,
        );
        assert!(res1.is_ok());

        // Second add with same branch fails (already checked out) -> exit code 27
        let res2 = add_worktree(
            &["same-branch".to_string(), "--ide=none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            false,
            None::<&mut Cursor<Vec<u8>>>,
        );
        assert!(res2.is_err());
        let err = res2.unwrap_err();
        assert_eq!(err.exit_code(), 27);
        assert!(err.to_string().contains("git worktree add command failed"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_add_worktree_with_ide_execution() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_add_ide_exec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = add_worktree(
            &[
                "feat-ide".to_string(),
                "--ide".to_string(),
                "touch created_by_add.txt".to_string(),
            ],
            Some(&repo_dir),
            Some(&config_dir),
            true,
            None::<&mut Cursor<Vec<u8>>>,
        );
        assert!(result.is_ok());

        let wt_path = temp_dir.join("gwt-myrepo").join("feat-ide");
        let marker_file = wt_path.join("created_by_add.txt");
        assert!(marker_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_add_error_exit_codes() {
        assert_eq!(AddError::MissingIdeArg.exit_code(), 23);
        assert_eq!(AddError::InvalidArgCount("".into()).exit_code(), 24);
        assert_eq!(AddError::DetermineTargetLocation.exit_code(), 25);
        assert_eq!(AddError::CreateParentDir("".into()).exit_code(), 26);
        assert_eq!(AddError::GitWorktreeAdd("".into()).exit_code(), 27);
        assert_eq!(AddError::CdWorktree("".into()).exit_code(), 28);
        assert_eq!(
            AddError::Io(io::Error::other("io err")).exit_code(),
            1
        );
    }
}
