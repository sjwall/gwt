use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::repos::get_current_main_repo;

/// Error types that can occur during the `remove` command.
#[derive(Debug)]
pub enum RemoveError {
    /// Cannot remove main repository (a worktree target must be specified) (exit code 13).
    CannotRemoveMainRepo,
    /// Failed to change directory to main repository (exit code 14).
    CdMainRepo(String),
    /// `git worktree remove` command failed (exit code 15).
    GitWorktreeRemove(String),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl RemoveError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            RemoveError::CannotRemoveMainRepo => 13,
            RemoveError::CdMainRepo(_) => 14,
            RemoveError::GitWorktreeRemove(_) => 15,
            RemoveError::Io(_) => 1,
        }
    }
}

impl fmt::Display for RemoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemoveError::CannotRemoveMainRepo => {
                write!(f, "cannot remove main repository; please specify a worktree")
            }
            RemoveError::CdMainRepo(msg) => {
                if msg.is_empty() {
                    write!(f, "failed to change directory to main repository")
                } else {
                    write!(f, "failed to change directory to main repository: {msg}")
                }
            }
            RemoveError::GitWorktreeRemove(msg) => {
                if msg.is_empty() {
                    write!(f, "git worktree remove command failed")
                } else {
                    write!(f, "git worktree remove command failed: {msg}")
                }
            }
            RemoveError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for RemoveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RemoveError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for RemoveError {
    fn from(err: io::Error) -> Self {
        RemoveError::Io(err)
    }
}

/// CLI arguments for the `remove` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct RemoveArgs {
    /// Force deletion of worktree even if dirty
    #[arg(short, long)]
    pub force: bool,

    /// Worktree names or paths to remove (defaults to current directory if omitted)
    pub targets: Vec<String>,
}

pub type RemoveParsedArgs = RemoveArgs;

/// Parses CLI arguments for the `remove` command, supporting `-f`, `--force`, and `-force`.
pub fn parse_remove_args(args: &[String]) -> RemoveArgs {
    let mut force = false;
    let mut targets = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-f" | "--force" | "-force" => {
                force = true;
            }
            _ => {
                targets.push(arg.clone());
            }
        }
    }

    RemoveArgs { force, targets }
}

/// Gets the git top-level directory for the current working directory or specified path.
pub fn get_show_toplevel(cwd: Option<&Path>) -> Option<PathBuf> {
    let mut cmd = Command::new("git");
    if let Some(dir) = cwd {
        cmd.arg("-C").arg(dir);
    }
    cmd.args(["rev-parse", "--show-toplevel"]);
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

/// Helper to check if two paths refer to different repositories/directories.
fn is_different_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return false;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca != cb,
        _ => true,
    }
}

/// Checks whether the current directory is a linked git worktree (not the main repository).
pub fn is_linked_worktree(cwd: Option<&Path>, main_repo: Option<&Path>) -> bool {
    let current_wt = get_show_toplevel(cwd);
    if let (Some(curr), Some(main)) = (&current_wt, main_repo) {
        if is_different_path(curr, main) {
            return true;
        }
    }

    let mut cmd1 = Command::new("git");
    if let Some(dir) = cwd {
        cmd1.arg("-C").arg(dir);
    }
    cmd1.args(["rev-parse", "--git-dir"]);
    let out1 = cmd1.output().ok();

    let mut cmd2 = Command::new("git");
    if let Some(dir) = cwd {
        cmd2.arg("-C").arg(dir);
    }
    cmd2.args(["rev-parse", "--git-common-dir"]);
    let out2 = cmd2.output().ok();

    if let (Some(o1), Some(o2)) = (out1, out2) {
        if o1.status.success() && o2.status.success() {
            let s1 = String::from_utf8_lossy(&o1.stdout).trim().to_string();
            let s2 = String::from_utf8_lossy(&o2.stdout).trim().to_string();
            if !s1.is_empty() && !s2.is_empty() && s1 != s2 {
                return true;
            }
        }
    }

    false
}

/// Removes one or more git worktrees according to the specified parsed arguments.
pub fn remove_worktree_args(
    parsed: &RemoveArgs,
    current_dir: Option<&Path>,
) -> Result<(), RemoveError> {
    let main_repo = get_current_main_repo(current_dir);

    let mut flags = Vec::new();
    if parsed.force {
        flags.push("--force");
    }

    let target_repo = match main_repo {
        None => {
            let mut git_cmd = Command::new("git");
            if let Some(dir) = current_dir {
                git_cmd.arg("-C").arg(dir);
            }
            git_cmd.args(["worktree", "remove"]);
            git_cmd.args(&flags);
            git_cmd.args(&parsed.targets);

            let status = git_cmd.status().map_err(|e| RemoveError::GitWorktreeRemove(e.to_string()))?;
            if !status.success() {
                return Err(RemoveError::GitWorktreeRemove(String::new()));
            }
            return Ok(());
        }
        Some(repo) => repo,
    };

    let linked = is_linked_worktree(current_dir, Some(&target_repo));

    let targets = if parsed.targets.is_empty() {
        if linked {
            let current_wt = get_show_toplevel(current_dir)
                .or_else(|| current_dir.map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("."));
            vec![current_wt.to_string_lossy().to_string()]
        } else {
            return Err(RemoveError::CannotRemoveMainRepo);
        }
    } else {
        let mut resolved_targets = Vec::new();
        for t in &parsed.targets {
            let target_path = if Path::new(t).is_relative() {
                if let Some(cd) = current_dir {
                    cd.join(t)
                } else {
                    PathBuf::from(t)
                }
            } else {
                PathBuf::from(t)
            };

            if target_path.is_dir() {
                let resolved = target_path
                    .canonicalize()
                    .unwrap_or(target_path)
                    .to_string_lossy()
                    .to_string();
                resolved_targets.push(resolved);
            } else {
                resolved_targets.push(t.clone());
            }
        }
        resolved_targets
    };

    if !target_repo.is_dir() {
        return Err(RemoveError::CdMainRepo(format!(
            "directory does not exist: {}",
            target_repo.display()
        )));
    }

    let mut git_cmd = Command::new("git");
    git_cmd.arg("-C").arg(&target_repo);
    git_cmd.args(["worktree", "remove"]);
    git_cmd.args(&flags);
    git_cmd.args(&targets);

    let status = git_cmd
        .status()
        .map_err(|e| RemoveError::GitWorktreeRemove(e.to_string()))?;

    if !status.success() {
        return Err(RemoveError::GitWorktreeRemove(String::new()));
    }

    Ok(())
}

/// Removes one or more git worktrees according to the specified arguments.
pub fn remove_worktree(
    args: &[String],
    current_dir: Option<&Path>,
) -> Result<(), RemoveError> {
    let parsed = parse_remove_args(args);
    remove_worktree_args(&parsed, current_dir)
}

/// Runs the `remove` command with parsed `RemoveArgs`.
pub fn run_args(args: &RemoveArgs) -> Result<(), RemoveError> {
    remove_worktree_args(args, None)
}

/// Runs the `remove` command with CLI arguments.
pub fn run(args: &[String]) -> Result<(), RemoveError> {
    remove_worktree(args, None)
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
    fn test_parse_remove_args() {
        let p1 = parse_remove_args(&["-f".into(), "my-wt".into()]);
        assert_eq!(
            p1,
            RemoveParsedArgs {
                force: true,
                targets: vec!["my-wt".into()],
            }
        );

        let p2 = parse_remove_args(&["--force".into(), "wt1".into(), "wt2".into()]);
        assert_eq!(
            p2,
            RemoveParsedArgs {
                force: true,
                targets: vec!["wt1".into(), "wt2".into()],
            }
        );

        let p3 = parse_remove_args(&["-force".into(), "wt3".into()]);
        assert_eq!(
            p3,
            RemoveParsedArgs {
                force: true,
                targets: vec!["wt3".into()],
            }
        );

        let p4 = parse_remove_args(&["wt4".into(), "-f".into()]);
        assert_eq!(
            p4,
            RemoveParsedArgs {
                force: true,
                targets: vec!["wt4".into()],
            }
        );

        let p5 = parse_remove_args(&[]);
        assert_eq!(
            p5,
            RemoveParsedArgs {
                force: false,
                targets: vec![],
            }
        );

        let p6 = parse_remove_args(&["-f".into()]);
        assert_eq!(
            p6,
            RemoveParsedArgs {
                force: true,
                targets: vec![],
            }
        );
    }

    #[test]
    fn test_remove_main_repo_without_target_fails() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_rm_main_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let res = remove_worktree(&[], Some(&repo_dir));
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.exit_code(), 13);
        assert_eq!(
            err.to_string(),
            "cannot remove main repository; please specify a worktree"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_remove_current_linked_worktree() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_rm_linked_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let wt_dir = temp_dir.join("gwt-repo").join("feat-1");
        add_worktree(&repo_dir, "feat-1", &wt_dir);
        assert!(wt_dir.exists());

        let res = remove_worktree(&[], Some(&wt_dir));
        assert!(res.is_ok());
        assert!(!wt_dir.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_remove_current_linked_worktree_with_force() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_rm_force_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let wt_dir = temp_dir.join("gwt-repo").join("feat-force");
        add_worktree(&repo_dir, "feat-force", &wt_dir);
        assert!(wt_dir.exists());

        // Modify a file to make it dirty
        fs::write(wt_dir.join("dirty.txt"), "uncommitted").unwrap();

        // Without -f, removal should fail
        let res_no_force = remove_worktree(&[], Some(&wt_dir));
        assert!(res_no_force.is_err());
        assert_eq!(res_no_force.unwrap_err().exit_code(), 15);
        assert!(wt_dir.exists());

        // With -f, removal should succeed
        let res_force = remove_worktree(&["-f".into()], Some(&wt_dir));
        assert!(res_force.is_ok());
        assert!(!wt_dir.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_remove_by_path_from_main_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_rm_path_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let wt_dir = temp_dir.join("gwt-repo").join("feat-path");
        add_worktree(&repo_dir, "feat-path", &wt_dir);
        assert!(wt_dir.exists());

        let res = remove_worktree(
            &[wt_dir.to_str().unwrap().to_string()],
            Some(&repo_dir),
        );
        assert!(res.is_ok());
        assert!(!wt_dir.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_remove_by_relative_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_rm_rel_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let wt_dir = temp_dir.join("gwt-repo").join("feat-rel");
        add_worktree(&repo_dir, "feat-rel", &wt_dir);
        assert!(wt_dir.exists());

        let res = remove_worktree(
            &["../gwt-repo/feat-rel".to_string()],
            Some(&repo_dir),
        );
        assert!(res.is_ok());
        assert!(!wt_dir.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_remove_nonexistent_target_fails() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_rm_nonexist_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        init_git_repo(&repo_dir);

        let res = remove_worktree(&["nonexistent-wt".to_string()], Some(&repo_dir));
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.exit_code(), 15);
        assert_eq!(err.to_string(), "git worktree remove command failed");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_remove_error_exit_codes() {
        assert_eq!(RemoveError::CannotRemoveMainRepo.exit_code(), 13);
        assert_eq!(RemoveError::CdMainRepo("".into()).exit_code(), 14);
        assert_eq!(RemoveError::GitWorktreeRemove("".into()).exit_code(), 15);
        assert_eq!(
            RemoveError::Io(io::Error::new(io::ErrorKind::Other, "io error")).exit_code(),
            1
        );
    }
}
