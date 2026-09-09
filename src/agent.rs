use std::fmt;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::config::{get_config_file, get_key_value, set_key_value};

/// Error types that can occur during agent operations.
#[derive(Debug)]
pub enum AgentError {
    /// No agent configured (exit code 46).
    NoAgentConfigured,
    /// Could not determine config directory (exit code 1).
    ConfigDirNotFound,
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
    /// Command execution failed.
    CommandFailed(String),
}

impl AgentError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            AgentError::NoAgentConfigured => 46,
            AgentError::ConfigDirNotFound => 1,
            AgentError::Io(_) => 1,
            AgentError::CommandFailed(_) => 1,
        }
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::NoAgentConfigured => write!(f, "no agent configured"),
            AgentError::ConfigDirNotFound => write!(f, "could not determine config directory"),
            AgentError::Io(err) => write!(f, "{err}"),
            AgentError::CommandFailed(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AgentError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AgentError {
    fn from(err: io::Error) -> Self {
        AgentError::Io(err)
    }
}

impl From<AgentError> for io::Error {
    fn from(err: AgentError) -> Self {
        match err {
            AgentError::Io(e) => e,
            other => io::Error::new(io::ErrorKind::Other, other.to_string()),
        }
    }
}

/// Returns the configured agent command or name.
///
/// Priority:
/// 1. `agent` setting in the `config` file
/// 2. `GWT_AGENT` environment variable
/// 3. `None` if unset
pub fn get_configured_agent(config_dir: Option<&Path>) -> Option<String> {
    if let Some(config_file) = get_config_file(config_dir) {
        if let Ok(Some(val)) = get_key_value(&config_file, "agent") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    if let Ok(env_agent) = std::env::var("GWT_AGENT") {
        let trimmed = env_agent.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Sets the configured agent in the configuration file.
pub fn set_configured_agent(agent: &str, config_dir: Option<&Path>) -> io::Result<()> {
    let config_file = match get_config_file(config_dir) {
        Some(file) => file,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine config directory",
            ));
        }
    };
    set_key_value(&config_file, "agent", agent)
}

/// Resolves the effective agent command to use given an optional CLI override (`--agent`).
///
/// Returns `None` if the effective agent is `"none"` (case-insensitive) or unset.
pub fn resolve_agent_command(
    override_agent: Option<&str>,
    config_dir: Option<&Path>,
) -> Option<String> {
    let agent = override_agent
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| get_configured_agent(config_dir));

    match agent {
        Some(ref a) if a.eq_ignore_ascii_case("none") => None,
        other => other,
    }
}

/// Prompts the user interactively for an agent command if not already configured.
/// Saves the entered agent to the configuration file under the key `agent`.
/// Returns `AgentError::NoAgentConfigured` (exit code 46) if the user provides an empty string.
pub fn prompt_for_agent<R: BufRead>(
    config_dir: Option<&Path>,
    mut prompt_reader: Option<&mut R>,
) -> Result<String, AgentError> {
    let input_opt = match prompt_reader.as_mut() {
        Some(reader) => {
            eprint!("Enter command to launch agent: ");
            let _ = io::stderr().flush();
            let mut user_input = String::new();
            if reader.read_line(&mut user_input).is_ok() {
                Some(user_input)
            } else {
                None
            }
        }
        None => inquire::Text::new("Enter command to launch agent:")
            .prompt()
            .ok(),
    };

    if let Some(user_input) = input_opt {
        let trimmed = user_input.trim();
        if !trimmed.is_empty() {
            set_configured_agent(trimmed, config_dir)?;
            return Ok(trimmed.to_string());
        }
    }

    Err(AgentError::NoAgentConfigured)
}

/// Launches the agent command in the specified directory, prompting if unset, unless the effective agent is "none".
pub fn launch_agent_with_reader<R: BufRead>(
    override_agent: Option<&str>,
    dir: &Path,
    config_dir: Option<&Path>,
    prompt_reader: Option<&mut R>,
) -> Result<(), AgentError> {
    let agent_cmd = match override_agent.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(cmd) => cmd.to_string(),
        None => match get_configured_agent(config_dir) {
            Some(cmd) => cmd,
            None => prompt_for_agent(config_dir, prompt_reader)?,
        },
    };

    if agent_cmd.eq_ignore_ascii_case("none") {
        return Ok(());
    }

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&agent_cmd)
        .current_dir(dir)
        .status()?;

    if !status.success() {
        return Err(AgentError::CommandFailed(format!(
            "Agent command '{agent_cmd}' failed with status: {status}"
        )));
    }

    Ok(())
}

/// Launches the agent command in the specified directory, prompting interactively if unset.
pub fn launch_agent(
    override_agent: Option<&str>,
    dir: &Path,
    config_dir: Option<&Path>,
) -> io::Result<()> {
    launch_agent_with_reader(override_agent, dir, config_dir, None::<&mut io::Empty>)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_get_configured_agent_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_unset_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = std::env::var("GWT_AGENT").ok();
        unsafe { std::env::remove_var("GWT_AGENT") };

        assert_eq!(get_configured_agent(Some(&temp_dir)), None);

        if let Some(v) = orig {
            unsafe { std::env::set_var("GWT_AGENT", v) };
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_configured_agent_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_env_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = std::env::var("GWT_AGENT").ok();
        unsafe { std::env::set_var("GWT_AGENT", "opencode") };

        assert_eq!(
            get_configured_agent(Some(&temp_dir)),
            Some("opencode".to_string())
        );

        match orig {
            Some(v) => unsafe { std::env::set_var("GWT_AGENT", v) },
            None => unsafe { std::env::remove_var("GWT_AGENT") },
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_set_and_get_configured_agent() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_set_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = std::env::var("GWT_AGENT").ok();
        unsafe { std::env::set_var("GWT_AGENT", "env-agent") };

        // Config file takes precedence over env var
        set_configured_agent("claude", Some(&temp_dir)).unwrap();
        assert_eq!(
            get_configured_agent(Some(&temp_dir)),
            Some("claude".to_string())
        );

        match orig {
            Some(v) => unsafe { std::env::set_var("GWT_AGENT", v) },
            None => unsafe { std::env::remove_var("GWT_AGENT") },
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_agent_command() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_resolve_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = std::env::var("GWT_AGENT").ok();
        unsafe { std::env::remove_var("GWT_AGENT") };

        // Unset
        assert_eq!(resolve_agent_command(None, Some(&temp_dir)), None);

        // Override
        assert_eq!(
            resolve_agent_command(Some("opencode"), Some(&temp_dir)),
            Some("opencode".to_string())
        );
        assert_eq!(resolve_agent_command(Some("none"), Some(&temp_dir)), None);
        assert_eq!(resolve_agent_command(Some("NONE"), Some(&temp_dir)), None);

        // Configured
        set_configured_agent("custom-agent", Some(&temp_dir)).unwrap();
        assert_eq!(
            resolve_agent_command(None, Some(&temp_dir)),
            Some("custom-agent".to_string())
        );

        // Configured none
        set_configured_agent("none", Some(&temp_dir)).unwrap();
        assert_eq!(resolve_agent_command(None, Some(&temp_dir)), None);

        if let Some(v) = orig {
            unsafe { std::env::set_var("GWT_AGENT", v) };
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_prompt_for_agent_success() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_prompt_ok_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut input = io::Cursor::new(b"my-agent-cmd\n");
        let result = prompt_for_agent(Some(&temp_dir), Some(&mut input)).unwrap();
        assert_eq!(result, "my-agent-cmd");

        // Verify it was saved to config
        assert_eq!(
            get_configured_agent(Some(&temp_dir)),
            Some("my-agent-cmd".to_string())
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_prompt_for_agent_empty_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_prompt_err_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut input = io::Cursor::new(b"   \n");
        let result = prompt_for_agent(Some(&temp_dir), Some(&mut input));
        match result {
            Err(AgentError::NoAgentConfigured) => {
                assert_eq!(AgentError::NoAgentConfigured.exit_code(), 46);
                assert_eq!(
                    AgentError::NoAgentConfigured.to_string(),
                    "no agent configured"
                );
            }
            _ => panic!("expected NoAgentConfigured error"),
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_launch_agent() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_agent_launch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Effective agent is "none" -> no-op success
        let res = launch_agent_with_reader(
            Some("none"),
            &temp_dir,
            Some(&temp_dir),
            None::<&mut io::Empty>,
        );
        assert!(res.is_ok());

        // Custom agent executes command in target directory
        let res = launch_agent_with_reader(
            Some("touch agent_launched.txt"),
            &temp_dir,
            Some(&temp_dir),
            None::<&mut io::Empty>,
        );
        assert!(res.is_ok());
        assert!(temp_dir.join("agent_launched.txt").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_agent_error_exit_codes() {
        assert_eq!(AgentError::NoAgentConfigured.exit_code(), 46);
        assert_eq!(AgentError::ConfigDirNotFound.exit_code(), 1);
        assert_eq!(
            AgentError::Io(io::Error::new(io::ErrorKind::Other, "err")).exit_code(),
            1
        );
        assert_eq!(AgentError::CommandFailed("fail".into()).exit_code(), 1);
    }
}
