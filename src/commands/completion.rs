use clap::Args;
use std::fmt;

/// CLI arguments for the `completion` command.
#[derive(Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionArgs {
    /// Shell to generate completions for (default: zsh)
    #[arg(value_name = "SHELL")]
    pub shell: Option<String>,
}

/// Errors that can occur during shell completion generation.
#[derive(Debug, PartialEq, Eq)]
pub enum CompletionError {
    /// Specified shell is not supported.
    UnsupportedShell(String),
}

impl CompletionError {
    /// Returns the associated process exit code.
    pub fn exit_code(&self) -> i32 {
        1
    }
}

impl fmt::Display for CompletionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompletionError::UnsupportedShell(s) => {
                write!(f, "unsupported shell: '{s}' (supported shells: zsh)")
            }
        }
    }
}

impl std::error::Error for CompletionError {}

/// Returns the autocompletion script for the requested shell.
pub fn execute_completion(shell: Option<&str>) -> Result<&'static str, CompletionError> {
    let s = shell.unwrap_or("zsh").trim().to_lowercase();
    match s.as_str() {
        "zsh" | "" => Ok(crate::shell::ZSH_COMPLETION),
        other => Err(CompletionError::UnsupportedShell(other.to_string())),
    }
}

/// Runs the `completion` command and prints the output to stdout.
pub fn run_args(args: &CompletionArgs) -> Result<(), CompletionError> {
    let script = execute_completion(args.shell.as_deref())?;
    print!("{script}");
    Ok(())
}

/// Runs the `completion` command with string arguments.
pub fn run(args: &[String]) -> Result<(), CompletionError> {
    let shell = args.first().map(|s| s.as_str());
    let script = execute_completion(shell)?;
    print!("{script}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_completion_default() {
        let script = execute_completion(None).unwrap();
        assert!(script.contains("#compdef gwt"));
        assert!(script.contains("_gwt()"));
    }

    #[test]
    fn test_execute_completion_zsh() {
        let script = execute_completion(Some("zsh")).unwrap();
        assert!(script.contains("#compdef gwt"));
        assert!(script.contains("_gwt_comp_worktrees"));
    }

    #[test]
    fn test_execute_completion_unsupported() {
        let err = execute_completion(Some("fish")).unwrap_err();
        assert_eq!(err, CompletionError::UnsupportedShell("fish".to_string()));
        assert_eq!(err.exit_code(), 1);
        assert_eq!(
            err.to_string(),
            "unsupported shell: 'fish' (supported shells: zsh)"
        );
    }
}
