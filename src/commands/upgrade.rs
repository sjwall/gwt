use std::fmt;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Args;

use crate::config::expand_tilde;
use crate::skills::{cmd_sync, find_skills_src, get_default_install_dir, Colors};

/// CLI arguments for the `upgrade` command.
#[derive(Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct UpgradeArgs {
    /// Installation or repository directory (overrides default search)
    #[arg(long = "dir", alias = "install-dir")]
    pub dir: Option<String>,

    /// Additional arguments passed through to `git pull`
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl UpgradeArgs {
    /// Creates a new `UpgradeArgs` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an `UpgradeArgs` with a custom directory.
    pub fn with_dir(dir: impl Into<String>) -> Self {
        Self {
            dir: Some(dir.into()),
            args: Vec::new(),
        }
    }
}

/// Error types that can occur during the `upgrade` command.
#[derive(Debug)]
pub enum UpgradeError {
    /// `gwt` repository not found at target directory (exit code 34).
    RepoNotFound(String),
    /// `git pull` command failed during upgrade (exit code 35).
    GitPullFailed(String),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl UpgradeError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            UpgradeError::RepoNotFound(_) => 34,
            UpgradeError::GitPullFailed(_) => 35,
            UpgradeError::Io(_) => 1,
        }
    }
}

impl fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpgradeError::RepoNotFound(path) => {
                write!(f, "repository not found at {path}")
            }
            UpgradeError::GitPullFailed(msg) => {
                if msg.is_empty() {
                    write!(f, "git pull command failed during upgrade")
                } else {
                    write!(f, "git pull command failed during upgrade: {msg}")
                }
            }
            UpgradeError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for UpgradeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UpgradeError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for UpgradeError {
    fn from(err: io::Error) -> Self {
        UpgradeError::Io(err)
    }
}

/// Checks if a directory appears to be the `gwt` repository.
fn is_gwt_repo(path: &Path) -> bool {
    if !path.join(".git").exists() {
        return false;
    }
    path.join("gwt.sh").exists()
        || path.join("Cargo.toml").exists()
        || path.join(".agents").join("skills").join("gwt").exists()
        || path.join("_gwt").exists()
        || path.join("_gwt_wrapper").exists()
}

/// Finds the `gwt` repository directory.
///
/// Priority:
/// 1. `custom_dir` if specified (e.g. `--dir`).
/// 2. `GWT_DIR` environment variable.
/// 3. Default install directory (`${XDG_DATA_HOME:-$HOME/.local/share}/gwt`).
/// 4. Current binary directory or ancestor directories containing `.git` and repository markers.
///
/// Returns `Ok(path)` if found, or `Err(target_path)` indicating where the repository was searched.
pub fn find_gwt_repo_dir(custom_dir: Option<&str>) -> Result<PathBuf, PathBuf> {
    if let Some(dir_str) = custom_dir {
        let path = expand_tilde(dir_str);
        if path.join(".git").exists() {
            return Ok(path);
        }
        return Err(path);
    }

    let env_gwt_dir = std::env::var("GWT_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| expand_tilde(&s));

    if let Some(ref path) = env_gwt_dir {
        if path.join(".git").exists() {
            return Ok(path.clone());
        }
        return Err(path.clone());
    }

    let default_dir = get_default_install_dir();
    if default_dir.join(".git").exists() {
        return Ok(default_dir);
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut candidates = vec![exe.clone()];
        if let Ok(canonical) = exe.canonicalize() {
            if canonical != exe {
                candidates.push(canonical);
            }
        }
        for cand in candidates {
            let mut cur = cand.parent();
            while let Some(parent) = cur {
                if is_gwt_repo(parent) {
                    return Ok(parent.to_path_buf());
                }
                cur = parent.parent();
            }
        }
    }

    Err(default_dir)
}

/// Default git pull runner that executes `git -C <repo_dir> pull <args...>`.
pub fn run_git_pull_default(repo_dir: &Path, args: &[String]) -> Result<bool, io::Error> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_dir);
    cmd.arg("-C").arg(repo_dir).arg("pull");
    cmd.args(args);
    let status = cmd.status()?;
    Ok(status.success())
}

/// Executes the `upgrade` command with a specified git pull runner and output writer.
pub fn execute_upgrade_with_git<W: Write, F>(
    args: &UpgradeArgs,
    out: &mut W,
    git_pull: F,
) -> Result<(), UpgradeError>
where
    F: FnOnce(&Path, &[String]) -> Result<bool, io::Error>,
{
    let gwt_dir = match find_gwt_repo_dir(args.dir.as_deref()) {
        Ok(dir) => dir,
        Err(target) => return Err(UpgradeError::RepoNotFound(target.display().to_string())),
    };

    writeln!(out, "Upgrading gwt at {}...", gwt_dir.display())?;

    let success =
        git_pull(&gwt_dir, &args.args).map_err(|e| UpgradeError::GitPullFailed(e.to_string()))?;

    if !success {
        return Err(UpgradeError::GitPullFailed(String::new()));
    }

    // Re-sync agent skill symlinks if skills exist
    let skills_src = find_skills_src(&gwt_dir);
    let is_terminal = io::stdout().is_terminal();
    let colors = Colors::new(is_terminal);
    let _ = cmd_sync(&skills_src, false, &colors, out);

    Ok(())
}

/// Executes the `upgrade` command writing output to `out`.
pub fn execute_upgrade<W: Write>(args: &UpgradeArgs, out: &mut W) -> Result<(), UpgradeError> {
    execute_upgrade_with_git(args, out, run_git_pull_default)
}

/// Runs the `upgrade` command using standard stdout.
pub fn run_args(args: &UpgradeArgs) -> Result<(), UpgradeError> {
    let mut stdout = io::stdout();
    execute_upgrade(args, &mut stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn init_git_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();
        let _ = Command::new("git")
            .current_dir(path)
            .args(["init", "-b", "main"])
            .output();
        let _ = Command::new("git")
            .current_dir(path)
            .args(["config", "user.name", "Test"])
            .output();
        let _ = Command::new("git")
            .current_dir(path)
            .args(["config", "user.email", "test@example.com"])
            .output();
        fs::write(path.join("README.md"), "initial").unwrap();
        let _ = Command::new("git")
            .current_dir(path)
            .args(["add", "."])
            .output();
        let _ = Command::new("git")
            .current_dir(path)
            .args(["commit", "-m", "init"])
            .output();
    }

    #[test]
    fn test_upgrade_args_parsing() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            command: TestCommands,
        }

        #[derive(clap::Subcommand)]
        enum TestCommands {
            Upgrade(UpgradeArgs),
        }

        let cli = TestCli::parse_from(["gwt", "upgrade"]);
        match cli.command {
            TestCommands::Upgrade(args) => {
                assert!(args.dir.is_none());
                assert!(args.args.is_empty());
            }
        }

        let cli = TestCli::parse_from(["gwt", "upgrade", "--dir", "/tmp/repo", "--rebase"]);
        match cli.command {
            TestCommands::Upgrade(args) => {
                assert_eq!(args.dir.as_deref(), Some("/tmp/repo"));
                assert_eq!(args.args, vec!["--rebase"]);
            }
        }

        let cli = TestCli::parse_from(["gwt", "upgrade", "origin", "main"]);
        match cli.command {
            TestCommands::Upgrade(args) => {
                assert!(args.dir.is_none());
                assert_eq!(args.args, vec!["origin", "main"]);
            }
        }
    }

    #[test]
    fn test_upgrade_error_display_and_exit_codes() {
        let err34 = UpgradeError::RepoNotFound("/path/to/repo".to_string());
        assert_eq!(err34.exit_code(), 34);
        assert_eq!(err34.to_string(), "repository not found at /path/to/repo");

        let err35_empty = UpgradeError::GitPullFailed(String::new());
        assert_eq!(err35_empty.exit_code(), 35);
        assert_eq!(
            err35_empty.to_string(),
            "git pull command failed during upgrade"
        );

        let err35_msg = UpgradeError::GitPullFailed("network error".to_string());
        assert_eq!(err35_msg.exit_code(), 35);
        assert_eq!(
            err35_msg.to_string(),
            "git pull command failed during upgrade: network error"
        );

        let io_err = UpgradeError::Io(io::Error::new(io::ErrorKind::Other, "io failure"));
        assert_eq!(io_err.exit_code(), 1);
        assert_eq!(io_err.to_string(), "io failure");
    }

    #[test]
    fn test_find_gwt_repo_dir_custom_dir() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_upgrade_custom_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        init_git_repo(&temp_dir);

        let found = find_gwt_repo_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        assert_eq!(found, temp_dir);

        let non_existent = temp_dir.join("nonexistent");
        let err = find_gwt_repo_dir(Some(non_existent.to_str().unwrap())).unwrap_err();
        assert_eq!(err, non_existent);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_gwt_repo_dir_env_var() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_upgrade_env_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        init_git_repo(&temp_dir);

        unsafe {
            std::env::set_var("GWT_DIR", &temp_dir);
        }

        let found = find_gwt_repo_dir(None).unwrap();
        assert_eq!(found, temp_dir);

        let non_existent = temp_dir.join("nonexistent");
        unsafe {
            std::env::set_var("GWT_DIR", &non_existent);
        }
        let err = find_gwt_repo_dir(None).unwrap_err();
        assert_eq!(err, non_existent);

        unsafe {
            std::env::remove_var("GWT_DIR");
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_execute_upgrade_repo_not_found_exit_34() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_upgrade_e34_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let args = UpgradeArgs::with_dir(temp_dir.to_str().unwrap());
        let mut out = Vec::new();

        let err = execute_upgrade(&args, &mut out).unwrap_err();
        assert_eq!(err.exit_code(), 34);
        assert!(err.to_string().contains("repository not found at"));
    }

    #[test]
    fn test_execute_upgrade_git_pull_failure_exit_35() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_upgrade_e35_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        init_git_repo(&temp_dir);

        let args = UpgradeArgs::with_dir(temp_dir.to_str().unwrap());
        let mut out = Vec::new();

        // Git pull returns false (command failure)
        let err = execute_upgrade_with_git(&args, &mut out, |_repo, _args| Ok(false)).unwrap_err();
        assert_eq!(err.exit_code(), 35);
        assert_eq!(err.to_string(), "git pull command failed during upgrade");
        assert!(String::from_utf8_lossy(&out).contains("Upgrading gwt at"));

        // Git pull returns IO error (executable not found / permission error)
        let mut out2 = Vec::new();
        let err2 = execute_upgrade_with_git(&args, &mut out2, |_repo, _args| {
            Err(io::Error::new(io::ErrorKind::NotFound, "git missing"))
        })
        .unwrap_err();
        assert_eq!(err2.exit_code(), 35);
        assert!(err2.to_string().contains("git missing"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_execute_upgrade_success_with_skills_sync() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_upgrade_success_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        init_git_repo(&temp_dir);

        // Create a fake skill in .agents/skills/gwt
        let skill_dir = temp_dir.join(".agents").join("skills").join("gwt");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# gwt skill").unwrap();

        let mut args = UpgradeArgs::with_dir(temp_dir.to_str().unwrap());
        args.args = vec!["--rebase".to_string()];
        let mut out = Vec::new();

        let mut recorded_args = Vec::new();
        let res = execute_upgrade_with_git(&args, &mut out, |_repo, extra_args| {
            recorded_args = extra_args.to_vec();
            Ok(true)
        });

        assert!(res.is_ok());
        assert_eq!(recorded_args, vec!["--rebase"]);
        let output_str = String::from_utf8_lossy(&out);
        assert!(output_str.contains("Upgrading gwt at"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_execute_upgrade_real_local_git_repos() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base_dir =
            std::env::temp_dir().join(format!("gwt_test_upgrade_real_{}_{}", std::process::id(), id));
        let _ = fs::remove_dir_all(&base_dir);
        fs::create_dir_all(&base_dir).unwrap();

        let upstream = base_dir.join("upstream");
        init_git_repo(&upstream);

        let downstream = base_dir.join("downstream");
        let clone_status = Command::new("git")
            .current_dir(&base_dir)
            .args([
                "clone",
                "-c",
                "protocol.file.allow=always",
                upstream.to_str().unwrap(),
                downstream.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            clone_status.status.success(),
            "git clone failed: {}",
            String::from_utf8_lossy(&clone_status.stderr)
        );

        // Add a commit in upstream
        fs::write(upstream.join("new_file.txt"), "new content").unwrap();
        let _ = Command::new("git")
            .current_dir(&upstream)
            .args(["add", "."])
            .output();
        let _ = Command::new("git")
            .current_dir(&upstream)
            .args(["commit", "-m", "upstream update"])
            .output();

        let args = UpgradeArgs::with_dir(downstream.to_str().unwrap());
        let mut out = Vec::new();
        let res = execute_upgrade(&args, &mut out);
        assert!(res.is_ok());

        assert!(downstream.join("new_file.txt").exists());
        let output_str = String::from_utf8_lossy(&out);
        assert!(output_str.contains("Upgrading gwt at"));

        let _ = fs::remove_dir_all(&base_dir);
    }
}
