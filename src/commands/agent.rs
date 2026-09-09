use std::fmt;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use crate::agent::{launch_agent_with_reader, AgentError};
use crate::commands::cd::{find_matching_worktree, CdError};

/// Error types that can occur during the `agent` command.
#[derive(Debug)]
pub enum AgentCmdError {
    /// Missing required argument for `--agent` option (exit code 44).
    MissingAgentArg,
    /// Missing required argument for `--ide` option (exit code 11).
    MissingIdeArg,
    /// Invalid argument count (expected exactly 1 worktree name) (exit code 45).
    InvalidArgCount(String),
    /// Error finding or resolving the worktree (exit codes 2, 3, 4, etc.).
    Cd(CdError),
    /// Error configuring or launching agent (exit code 46, etc.).
    Agent(AgentError),
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl AgentCmdError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            AgentCmdError::MissingAgentArg => 44,
            AgentCmdError::MissingIdeArg => 11,
            AgentCmdError::InvalidArgCount(_) => 45,
            AgentCmdError::Cd(err) => err.exit_code(),
            AgentCmdError::Agent(err) => err.exit_code(),
            AgentCmdError::Io(_) => 1,
        }
    }
}

impl fmt::Display for AgentCmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentCmdError::MissingAgentArg => write!(f, "--agent requires an argument"),
            AgentCmdError::MissingIdeArg => write!(f, "--ide requires an argument"),
            AgentCmdError::InvalidArgCount(args) => {
                if args.is_empty() {
                    write!(f, "unknown command: agent ''")
                } else {
                    write!(f, "unknown command: agent '{args}'")
                }
            }
            AgentCmdError::Cd(err) => write!(f, "{err}"),
            AgentCmdError::Agent(err) => write!(f, "{err}"),
            AgentCmdError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for AgentCmdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AgentCmdError::Cd(err) => Some(err),
            AgentCmdError::Agent(err) => Some(err),
            AgentCmdError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<CdError> for AgentCmdError {
    fn from(err: CdError) -> Self {
        AgentCmdError::Cd(err)
    }
}

impl From<AgentError> for AgentCmdError {
    fn from(err: AgentError) -> Self {
        AgentCmdError::Agent(err)
    }
}

impl From<io::Error> for AgentCmdError {
    fn from(err: io::Error) -> Self {
        AgentCmdError::Io(err)
    }
}

/// CLI arguments for the `agent` / `a` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentArgs {
    /// Arguments for agent command (worktree name and optional --agent <AGENT>)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl AgentArgs {
    /// Creates a new `AgentArgs` with raw arguments.
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }
}

/// Parsed options and arguments for the `agent` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentParsedArgs {
    /// Worktree name query to match
    pub query: String,
    /// Optional agent override (e.g. from `--agent <agent>`)
    pub agent: Option<String>,
}

/// Parses CLI arguments for the `agent` command, supporting `--agent <AGENT>`, `--agent=<AGENT>`,
/// and handling `--ide` compatibility.
pub fn parse_agent_args(args: &[String]) -> Result<AgentParsedArgs, AgentCmdError> {
    let mut override_agent = None;
    let mut positional = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if arg == "--agent" {
            if i + 1 >= args.len() {
                return Err(AgentCmdError::MissingAgentArg);
            }
            override_agent = Some(args[i + 1].clone());
            i += 2;
        } else if let Some(val) = arg.strip_prefix("--agent=") {
            override_agent = Some(val.to_string());
            i += 1;
        } else if arg == "--ide" {
            if i + 1 >= args.len() {
                return Err(AgentCmdError::MissingIdeArg);
            }
            i += 2;
        } else if arg.starts_with("--ide=") {
            i += 1;
        } else {
            positional.push(arg.clone());
            i += 1;
        }
    }

    if positional.len() != 1 {
        return Err(AgentCmdError::InvalidArgCount(positional.join(" ")));
    }

    let query = positional[0].trim().to_string();
    if query.is_empty() {
        return Err(AgentCmdError::InvalidArgCount(String::new()));
    }

    Ok(AgentParsedArgs {
        query,
        agent: override_agent,
    })
}

/// Resolves the worktree for the `agent` command with parsed `AgentParsedArgs` and optionally launches the agent.
/// Returns the matched worktree path on success.
pub fn agent_worktree_args<R: BufRead>(
    parsed: &AgentParsedArgs,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
    prompt_reader: Option<&mut R>,
) -> Result<PathBuf, AgentCmdError> {
    let path = find_matching_worktree(&parsed.query, current_dir, config_dir)?;

    if launch {
        launch_agent_with_reader(
            parsed.agent.as_deref(),
            &path,
            config_dir,
            prompt_reader,
        )?;
    }

    Ok(path)
}

/// Resolves the worktree for the `agent` command and optionally launches the agent.
/// Returns the matched worktree path on success.
pub fn agent_worktree<R: BufRead>(
    args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
    prompt_reader: Option<&mut R>,
) -> Result<PathBuf, AgentCmdError> {
    let parsed = parse_agent_args(args)?;
    agent_worktree_args(&parsed, current_dir, config_dir, launch, prompt_reader)
}

/// Runs the `agent` command with parsed `AgentArgs`.
pub fn run_args(args: &AgentArgs) -> Result<PathBuf, AgentCmdError> {
    agent_worktree(
        &args.args,
        None,
        None,
        true,
        None::<&mut io::Empty>,
    )
}

/// Runs the `agent` command with CLI arguments.
pub fn run(args: &[String]) -> Result<PathBuf, AgentCmdError> {
    agent_worktree(args, None, None, true, None::<&mut io::Empty>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
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
    fn test_parse_agent_args() {
        let p1 = parse_agent_args(&["--agent".into(), "opencode".into(), "my-branch".into()]).unwrap();
        assert_eq!(
            p1,
            AgentParsedArgs {
                query: "my-branch".into(),
                agent: Some("opencode".into()),
            }
        );

        let p2 = parse_agent_args(&["--agent=claude".into(), "feat-x".into()]).unwrap();
        assert_eq!(
            p2,
            AgentParsedArgs {
                query: "feat-x".into(),
                agent: Some("claude".into()),
            }
        );

        let p3 = parse_agent_args(&["feat-y".into(), "--agent=none".into()]).unwrap();
        assert_eq!(
            p3,
            AgentParsedArgs {
                query: "feat-y".into(),
                agent: Some("none".into()),
            }
        );

        let p4 = parse_agent_args(&["feat-z".into()]).unwrap();
        assert_eq!(
            p4,
            AgentParsedArgs {
                query: "feat-z".into(),
                agent: None,
            }
        );

        let p5 = parse_agent_args(&["--ide=code".into(), "feat-w".into()]).unwrap();
        assert_eq!(
            p5,
            AgentParsedArgs {
                query: "feat-w".into(),
                agent: None,
            }
        );
    }

    #[test]
    fn test_agent_arg_validation() {
        let err_empty = parse_agent_args(&[]).unwrap_err();
        assert_eq!(err_empty.exit_code(), 45);
        assert_eq!(err_empty.to_string(), "unknown command: agent ''");

        let err_multi = parse_agent_args(&["feat1".into(), "feat2".into()]).unwrap_err();
        assert_eq!(err_multi.exit_code(), 45);
        assert_eq!(err_multi.to_string(), "unknown command: agent 'feat1 feat2'");

        let err_blank = parse_agent_args(&["   ".into()]).unwrap_err();
        assert_eq!(err_blank.exit_code(), 45);
        assert_eq!(err_blank.to_string(), "unknown command: agent ''");

        let err_missing_agent = parse_agent_args(&["--agent".into()]).unwrap_err();
        assert_eq!(err_missing_agent.exit_code(), 44);
        assert_eq!(err_missing_agent.to_string(), "--agent requires an argument");

        let err_missing_agent_after = parse_agent_args(&["feat".into(), "--agent".into()]).unwrap_err();
        assert_eq!(err_missing_agent_after.exit_code(), 44);
        assert_eq!(err_missing_agent_after.to_string(), "--agent requires an argument");

        let err_missing_ide = parse_agent_args(&["--ide".into()]).unwrap_err();
        assert_eq!(err_missing_ide.exit_code(), 11);
        assert_eq!(err_missing_ide.to_string(), "--ide requires an argument");
    }

    #[test]
    fn test_agent_worktree_with_none_suppressed() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_suppress_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feature-one");
        add_worktree(&repo_dir, "feature-one", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = agent_worktree(
            &["feature-one".to_string(), "--agent".to_string(), "none".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
            None::<&mut Cursor<Vec<u8>>>,
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            wt_feat.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_agent_worktree_with_execution() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_exec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feature-exec");
        add_worktree(&repo_dir, "feature-exec", &wt_feat);

        let config_dir = temp_dir.join("config");
        let result = agent_worktree(
            &[
                "feature-exec".to_string(),
                "--agent".to_string(),
                "touch agent_switched.txt".to_string(),
            ],
            Some(&repo_dir),
            Some(&config_dir),
            true,
            None::<&mut Cursor<Vec<u8>>>,
        );
        assert!(result.is_ok());

        let marker = wt_feat.join("agent_switched.txt");
        assert!(marker.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_agent_worktree_prompt_saves_and_runs() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_prompt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feat-prompt");
        add_worktree(&repo_dir, "feat-prompt", &wt_feat);

        let config_dir = temp_dir.join("config");
        let mut prompt_input = Cursor::new(b"touch prompt_agent.txt\n".to_vec());

        let result = agent_worktree(
            &["feat-prompt".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
            Some(&mut prompt_input),
        );
        assert!(result.is_ok());

        let marker = wt_feat.join("prompt_agent.txt");
        assert!(marker.exists());

        // Check agent was saved to config
        assert_eq!(
            crate::agent::get_configured_agent(Some(&config_dir)),
            Some("touch prompt_agent.txt".to_string())
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_agent_worktree_empty_prompt_gives_code_46() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_empty_p_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_feat = temp_dir.join("gwt-myrepo").join("feat-empty");
        add_worktree(&repo_dir, "feat-empty", &wt_feat);

        let config_dir = temp_dir.join("config");
        let mut prompt_input = Cursor::new(b"   \n".to_vec());

        let result = agent_worktree(
            &["feat-empty".to_string()],
            Some(&repo_dir),
            Some(&config_dir),
            true,
            Some(&mut prompt_input),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 46);
        assert_eq!(err.to_string(), "no agent configured");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_agent_error_exit_codes() {
        assert_eq!(AgentCmdError::MissingAgentArg.exit_code(), 44);
        assert_eq!(AgentCmdError::MissingIdeArg.exit_code(), 11);
        assert_eq!(AgentCmdError::InvalidArgCount("".into()).exit_code(), 45);
        assert_eq!(AgentCmdError::Agent(AgentError::NoAgentConfigured).exit_code(), 46);
        assert_eq!(AgentCmdError::Cd(CdError::NoMatch("x".into())).exit_code(), 4);
        assert_eq!(
            AgentCmdError::Io(io::Error::new(io::ErrorKind::Other, "io err")).exit_code(),
            1
        );
    }
}
