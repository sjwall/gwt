use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::locations::determine_dir_gwt;
use crate::repos::get_current_main_repo;
use crate::worktree::parse_porcelain;

/// Error types that can occur during the `migrate` command.
#[derive(Debug)]
pub enum MigrateError {
    /// Not inside a git repository (exit code 48).
    NotInsideGitRepo,
    /// Unknown argument or option passed (exit code 49).
    UnknownCommand(String),
    /// Failed to determine target worktree directory location (exit code 50).
    DetermineTargetLocation,
    /// `git worktree move` command failed (exit code 51).
    GitWorktreeMove { src: String, dest: String },
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl MigrateError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            MigrateError::NotInsideGitRepo => 48,
            MigrateError::UnknownCommand(_) => 49,
            MigrateError::DetermineTargetLocation => 50,
            MigrateError::GitWorktreeMove { .. } => 51,
            MigrateError::Io(_) => 1,
        }
    }
}

impl fmt::Display for MigrateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrateError::NotInsideGitRepo => write!(f, "not inside a git repository"),
            MigrateError::UnknownCommand(args) => write!(f, "unknown command: 'migrate {args}'"),
            MigrateError::DetermineTargetLocation => {
                write!(f, "failed to determine target worktree directory location")
            }
            MigrateError::GitWorktreeMove { src, dest } => {
                write!(f, "failed to move worktree '{src}' to '{dest}'")
            }
            MigrateError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for MigrateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MigrateError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for MigrateError {
    fn from(err: io::Error) -> Self {
        MigrateError::Io(err)
    }
}

/// CLI arguments for the `migrate` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct MigrateArgs {
    /// Arguments for migrate command (supports -n/--dry-run, -f/--force/-force)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl MigrateArgs {
    /// Creates a new `MigrateArgs` with raw arguments.
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    /// Creates a new `MigrateArgs` with specified flags.
    pub fn from_flags(dry_run: bool, force: bool) -> Self {
        let mut args = Vec::new();
        if dry_run {
            args.push("--dry-run".to_string());
        }
        if force {
            args.push("--force".to_string());
        }
        Self { args }
    }
}

/// Parsed options for the `migrate` command.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MigrateParsedArgs {
    /// Whether to perform a dry run without making filesystem changes
    pub dry_run: bool,
    /// Whether to pass `--force` to `git worktree move`
    pub force: bool,
}

/// Parses CLI arguments for the `migrate` command.
///
/// Supports `-n` / `--dry-run`, and `-f` / `--force` / `-force`.
/// Returns `MigrateError::UnknownCommand` (exit code 49) if any unexpected arguments are found.
pub fn parse_migrate_args(args: &[String]) -> Result<MigrateParsedArgs, MigrateError> {
    let mut dry_run = false;
    let mut force = false;
    let mut unknown_args = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-n" | "--dry-run" => {
                dry_run = true;
            }
            "-f" | "--force" | "-force" => {
                force = true;
            }
            _ => {
                unknown_args.push(arg.clone());
            }
        }
    }

    if !unknown_args.is_empty() {
        return Err(MigrateError::UnknownCommand(unknown_args.join(" ")));
    }

    Ok(MigrateParsedArgs { dry_run, force })
}

/// Helper to canonicalize a path if possible, or canonicalize its parent if the path doesn't exist yet.
pub fn canonicalize_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else if let Some(parent) = path.parent() {
        if let Ok(can_parent) = parent.canonicalize() {
            if let Some(file_name) = path.file_name() {
                can_parent.join(file_name)
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    }
}

/// Migrates worktrees of the current repository to match the expected directory pattern.
///
/// Returns the new directory path if the current working directory was inside a moved worktree.
pub fn migrate_worktrees_args<R: io::BufRead>(
    parsed: &MigrateParsedArgs,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    prompt_reader: Option<&mut R>,
) -> Result<Option<PathBuf>, MigrateError> {
    migrate_worktrees(
        parsed.dry_run,
        parsed.force,
        current_dir,
        config_dir,
        prompt_reader,
    )
}

/// Migrates worktrees of the current repository to match the expected directory pattern.
pub fn migrate_worktrees<R: io::BufRead>(
    dry_run: bool,
    force: bool,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    prompt_reader: Option<&mut R>,
) -> Result<Option<PathBuf>, MigrateError> {
    let main_repo = get_current_main_repo(current_dir).ok_or(MigrateError::NotInsideGitRepo)?;

    let dir_gwt = determine_dir_gwt(&main_repo, config_dir, prompt_reader)
        .ok_or(MigrateError::DetermineTargetLocation)?;

    let target_base = canonicalize_path(&dir_gwt);

    // Prune stale worktrees first
    let _ = Command::new("git")
        .arg("-C")
        .arg(&main_repo)
        .args(["worktree", "prune"])
        .output();

    let output = Command::new("git")
        .arg("-C")
        .arg(&main_repo)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(MigrateError::Io)?;

    if !output.status.success() {
        return Ok(None);
    }

    let wt_output = String::from_utf8_lossy(&output.stdout);
    let worktrees = parse_porcelain(&wt_output);

    let can_main = canonicalize_path(&main_repo);

    let mut to_move = Vec::new();

    for wt in &worktrees {
        let can_wt = canonicalize_path(&wt.path);
        if can_wt == can_main {
            continue;
        }
        if !can_wt.is_dir() {
            continue;
        }

        let expected_dest = if let Some(ref branch) = wt.branch {
            if !branch.is_empty() {
                target_base.join(branch)
            } else {
                let name = can_wt.file_name().unwrap_or_default();
                target_base.join(name)
            }
        } else {
            let name = can_wt.file_name().unwrap_or_default();
            target_base.join(name)
        };

        let can_expected = canonicalize_path(&expected_dest);

        if can_wt != can_expected {
            to_move.push((can_wt, expected_dest));
        }
    }

    if to_move.is_empty() {
        eprintln!("gwt: all worktrees match the expected path pattern");
        return Ok(None);
    }

    if dry_run {
        for (src, dest) in &to_move {
            eprintln!("Would move '{}' to '{}'", src.display(), dest.display());
        }
        return Ok(None);
    }

    let cwd = current_dir
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());

    let orig_pwd = cwd.as_ref().map(|p| canonicalize_path(p));

    let mut new_pwd: Option<PathBuf> = None;
    let mut fallback_dest: Option<PathBuf> = None;

    for (src, dest) in &to_move {
        eprintln!("Moving '{}' to '{}'...", src.display(), dest.display());

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(MigrateError::Io)?;
        }

        let mut move_cmd = Command::new("git");
        move_cmd.arg("-C").arg(&main_repo);
        move_cmd.args(["worktree", "move"]);
        if force {
            move_cmd.arg("--force");
        }
        move_cmd.arg(src).arg(dest);

        let status = move_cmd
            .status()
            .map_err(|_| MigrateError::GitWorktreeMove {
                src: src.display().to_string(),
                dest: dest.display().to_string(),
            })?;

        if !status.success() {
            return Err(MigrateError::GitWorktreeMove {
                src: src.display().to_string(),
                dest: dest.display().to_string(),
            });
        }

        if let Some(ref orig) = orig_pwd {
            if orig == src {
                new_pwd = Some(dest.clone());
                fallback_dest = Some(dest.clone());
            } else if let Ok(rel) = orig.strip_prefix(src) {
                new_pwd = Some(dest.join(rel));
                fallback_dest = Some(dest.clone());
            }
        }
    }

    if let Some(ref target_dir) = new_pwd {
        crate::shell::notify_cd_target(target_dir);
        if target_dir.is_dir() {
            let _ = std::env::set_current_dir(target_dir);
        } else if let Some(ref fb) = fallback_dest {
            crate::shell::notify_cd_target(fb);
            if fb.is_dir() {
                let _ = std::env::set_current_dir(fb);
            }
        }
    }

    Ok(new_pwd)
}

/// Runs the `migrate` command with parsed `MigrateArgs`.
pub fn run_args(args: &MigrateArgs) -> Result<Option<PathBuf>, MigrateError> {
    let parsed = parse_migrate_args(&args.args)?;
    let target = migrate_worktrees_args(&parsed, None, None, None::<&mut io::Empty>)?;
    if let Some(ref t) = target {
        println!("{}", t.display());
    }
    Ok(target)
}

/// Runs the `migrate` command with raw argument slice.
pub fn run(args: &[String]) -> Result<Option<PathBuf>, MigrateError> {
    let parsed = parse_migrate_args(args)?;
    let target = migrate_worktrees_args(&parsed, None, None, None::<&mut io::Empty>)?;
    if let Some(ref t) = target {
        println!("{}", t.display());
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::process::Command;

    use crate::locations::save_configured_parent;

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
    fn test_parse_migrate_args_empty() {
        let parsed = parse_migrate_args(&[]).unwrap();
        assert!(!parsed.dry_run);
        assert!(!parsed.force);
    }

    #[test]
    fn test_parse_migrate_args_dry_run_short() {
        let parsed = parse_migrate_args(&["-n".to_string()]).unwrap();
        assert!(parsed.dry_run);
        assert!(!parsed.force);
    }

    #[test]
    fn test_parse_migrate_args_dry_run_long() {
        let parsed = parse_migrate_args(&["--dry-run".to_string()]).unwrap();
        assert!(parsed.dry_run);
        assert!(!parsed.force);
    }

    #[test]
    fn test_parse_migrate_args_force_variants() {
        for flag in &["-f", "--force", "-force"] {
            let parsed = parse_migrate_args(&[flag.to_string()]).unwrap();
            assert!(!parsed.dry_run);
            assert!(parsed.force);
        }
    }

    #[test]
    fn test_parse_migrate_args_both_flags() {
        let parsed = parse_migrate_args(&["-n".to_string(), "--force".to_string()]).unwrap();
        assert!(parsed.dry_run);
        assert!(parsed.force);

        let parsed2 = parse_migrate_args(&["-force".to_string(), "--dry-run".to_string()]).unwrap();
        assert!(parsed2.dry_run);
        assert!(parsed2.force);
    }

    #[test]
    fn test_parse_migrate_args_unknown_arg_exit_code_49() {
        let err = parse_migrate_args(&["foo".to_string()]).unwrap_err();
        assert_eq!(err.exit_code(), 49);
        assert_eq!(err.to_string(), "unknown command: 'migrate foo'");

        let err2 = parse_migrate_args(&["--unknown".to_string()]).unwrap_err();
        assert_eq!(err2.exit_code(), 49);
        assert_eq!(err2.to_string(), "unknown command: 'migrate --unknown'");

        let err3 = parse_migrate_args(&["-n".to_string(), "arg1".to_string(), "arg2".to_string()]).unwrap_err();
        assert_eq!(err3.exit_code(), 49);
        assert_eq!(err3.to_string(), "unknown command: 'migrate arg1 arg2'");
    }

    #[test]
    fn test_migrate_error_display_and_exit_codes() {
        let e48 = MigrateError::NotInsideGitRepo;
        assert_eq!(e48.exit_code(), 48);
        assert_eq!(e48.to_string(), "not inside a git repository");

        let e49 = MigrateError::UnknownCommand("bogus".to_string());
        assert_eq!(e49.exit_code(), 49);
        assert_eq!(e49.to_string(), "unknown command: 'migrate bogus'");

        let e50 = MigrateError::DetermineTargetLocation;
        assert_eq!(e50.exit_code(), 50);
        assert_eq!(
            e50.to_string(),
            "failed to determine target worktree directory location"
        );

        let e51 = MigrateError::GitWorktreeMove {
            src: "/src".to_string(),
            dest: "/dest".to_string(),
        };
        assert_eq!(e51.exit_code(), 51);
        assert_eq!(
            e51.to_string(),
            "failed to move worktree '/src' to '/dest'"
        );

        let io_err = MigrateError::Io(io::Error::other("disk error"));
        assert_eq!(io_err.exit_code(), 1);
        assert_eq!(io_err.to_string(), "disk error");
    }

    #[test]
    fn test_migrate_not_inside_git_repo_exit_code_48() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_nongit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let result = migrate_worktrees(
            false,
            false,
            Some(&temp_dir),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 48);
        assert_eq!(err.to_string(), "not inside a git repository");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_unsuitable_location_without_input_exit_code_50() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_unsuitable_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let unsuitable_repo = temp_dir.join(".hidden_repo");
        init_git_repo(&unsuitable_repo);

        let mut empty_reader = Cursor::new("");
        let result = migrate_worktrees(
            false,
            false,
            Some(&unsuitable_repo),
            None,
            Some(&mut empty_reader),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 50);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_all_worktrees_match_when_none_exist() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_all_match_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let result = migrate_worktrees(
            false,
            false,
            Some(&repo_dir),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_dry_run_does_not_move() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_dryrun_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        // Add a worktree in an unexpected location
        let unexpected_wt = temp_dir.join("other_place").join("feature-1");
        fs::create_dir_all(unexpected_wt.parent().unwrap()).unwrap();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["worktree", "add", "-b", "feature-1", unexpected_wt.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());
        assert!(unexpected_wt.is_dir());

        // Run dry-run
        let result = migrate_worktrees(
            true,
            false,
            Some(&repo_dir),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result.is_ok());

        // Worktree should still be in the unexpected location
        assert!(unexpected_wt.is_dir());

        let expected_dest = temp_dir.join("gwt-repo").join("feature-1");
        assert!(!expected_dest.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_moves_worktree_to_expected_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_move_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        // Add a worktree in an unexpected location
        let unexpected_wt = temp_dir.join("custom_wts").join("feat-x");
        fs::create_dir_all(unexpected_wt.parent().unwrap()).unwrap();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["worktree", "add", "-b", "feat-x", unexpected_wt.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());
        assert!(unexpected_wt.is_dir());

        // Run real migration
        let result = migrate_worktrees(
            false,
            false,
            Some(&repo_dir),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result.is_ok());

        let expected_dest = temp_dir.join("gwt-repo").join("feat-x");
        assert!(expected_dest.is_dir());
        assert!(!unexpected_wt.exists());

        // Running migrate again should find all worktrees match
        let result2 = migrate_worktrees(
            false,
            false,
            Some(&repo_dir),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), None);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_moves_worktree_with_slash_branch() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_slash_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        // Add a worktree in an unexpected location with slashes in branch name
        let unexpected_wt = temp_dir.join("custom_wts").join("user-auth");
        fs::create_dir_all(unexpected_wt.parent().unwrap()).unwrap();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["worktree", "add", "-b", "feat/user-auth", unexpected_wt.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());

        // Run real migration
        let result = migrate_worktrees(
            false,
            false,
            Some(&repo_dir),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result.is_ok());

        let expected_dest = temp_dir.join("gwt-repo").join("feat").join("user-auth");
        assert!(expected_dest.is_dir());
        assert!(!unexpected_wt.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_moves_detached_worktree() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_detached_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        // Add a detached worktree in an unexpected location
        let unexpected_wt = temp_dir.join("custom_wts").join("detached-wt");
        fs::create_dir_all(unexpected_wt.parent().unwrap()).unwrap();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["worktree", "add", "--detach", unexpected_wt.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());

        // Run real migration
        let result = migrate_worktrees(
            false,
            false,
            Some(&repo_dir),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result.is_ok());

        let expected_dest = temp_dir.join("gwt-repo").join("detached-wt");
        assert!(expected_dest.is_dir());
        assert!(!unexpected_wt.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_updates_current_dir_when_inside_moved_worktree() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_cwd_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let unexpected_wt = temp_dir.join("custom_wts").join("feat-inside");
        fs::create_dir_all(unexpected_wt.parent().unwrap()).unwrap();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["worktree", "add", "-b", "feat-inside", unexpected_wt.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());

        // Create a subfolder inside the worktree
        let subfolder = unexpected_wt.join("src").join("utils");
        fs::create_dir_all(&subfolder).unwrap();

        // Migrate while passing subfolder as current_dir
        let result = migrate_worktrees(
            false,
            false,
            Some(&subfolder),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result.is_ok());

        let new_path = result.unwrap();
        assert!(new_path.is_some());
        let expected_new_sub = temp_dir.join("gwt-repo").join("feat-inside").join("src").join("utils");
        assert_eq!(
            canonicalize_path(&new_path.unwrap()),
            canonicalize_path(&expected_new_sub)
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_configured_safe_parent() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_cfg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let safe_parent = temp_dir.join("custom_safe");
        fs::create_dir_all(&safe_parent).unwrap();
        save_configured_parent(&repo_dir, &safe_parent, Some(&config_dir)).unwrap();

        // Worktree in unexpected location
        let unexpected_wt = temp_dir.join("unexpected").join("feat-safe");
        fs::create_dir_all(unexpected_wt.parent().unwrap()).unwrap();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["worktree", "add", "-b", "feat-safe", unexpected_wt.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());

        // Migrate using config_dir
        let result = migrate_worktrees(
            false,
            false,
            Some(&repo_dir),
            Some(&config_dir),
            None::<&mut io::Empty>,
        );
        assert!(result.is_ok());

        let expected_dest = safe_parent.join("gwt-repo").join("feat-safe");
        assert!(expected_dest.is_dir());
        assert!(!unexpected_wt.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_git_worktree_move_failure_returns_exit_code_51() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_migrate_fail51_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let unexpected_wt = temp_dir.join("custom").join("feat-fail");
        fs::create_dir_all(unexpected_wt.parent().unwrap()).unwrap();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["worktree", "add", "-b", "feat-fail", unexpected_wt.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());

        // Lock the worktree so git worktree move fails without --force
        let lock_status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["worktree", "lock", unexpected_wt.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(lock_status.success());

        let result = migrate_worktrees(
            false,
            false,
            Some(&repo_dir),
            None,
            None::<&mut io::Empty>,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 51);
        assert!(err.to_string().contains("failed to move worktree '"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrate_run_and_run_args() {
        let args = vec!["-n".to_string()];
        let migrate_args = MigrateArgs::new(args);
        assert_eq!(migrate_args.args, vec!["-n"]);

        let from_flags = MigrateArgs::from_flags(true, true);
        assert_eq!(from_flags.args, vec!["--dry-run", "--force"]);
    }
}
