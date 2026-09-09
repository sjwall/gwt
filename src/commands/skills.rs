use std::fmt;
use std::io::{self, IsTerminal};

use clap::Args;

use crate::config::expand_tilde;
use crate::skills::{
    apply_all_targets, cmd_list, cmd_prompt, cmd_sync, find_skills_src, get_default_install_dir,
    parse_skills_selection, Colors,
};

/// Arguments for `gwt skills`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct SkillsArgs {
    /// Action or targets to link (e.g. sync, list, prompt, claude, gemini, all, none)
    #[arg(num_args = 0..)]
    pub targets: Vec<String>,

    /// Installation directory (default: ~/.local/share/gwt)
    #[arg(long = "dir", alias = "install-dir")]
    pub dir: Option<String>,

    /// Specific skills selection
    #[arg(long = "skills")]
    pub skills: Option<String>,

    /// Skip skills symlinking (same as none)
    #[arg(long = "no-skills")]
    pub no_skills: bool,

    /// Update symlinks for currently linked agents
    #[arg(long = "sync")]
    pub sync: bool,

    /// List current symlink status for all agents
    #[arg(long = "list")]
    pub list: bool,

    /// Show interactive selection prompt
    #[arg(long = "prompt")]
    pub prompt: bool,

    /// Perform a dry run without making any changes
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Debug)]
pub enum SkillsError {
    /// Skills helper / directory not found (exit code 47)
    NotFound(String),
    /// An I/O error occurred (exit code 1)
    Io(io::Error),
    /// Argument error (exit code 1)
    InvalidArg(String),
}

impl SkillsError {
    pub fn exit_code(&self) -> i32 {
        match self {
            SkillsError::NotFound(_) => 47,
            SkillsError::Io(_) => 1,
            SkillsError::InvalidArg(_) => 1,
        }
    }
}

impl fmt::Display for SkillsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillsError::NotFound(msg) => write!(f, "skills helper not found: {msg}"),
            SkillsError::Io(err) => write!(f, "{err}"),
            SkillsError::InvalidArg(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SkillsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SkillsError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for SkillsError {
    fn from(err: io::Error) -> Self {
        SkillsError::Io(err)
    }
}

pub fn run_args(args: &SkillsArgs) -> Result<(), SkillsError> {
    execute_skills(args)
}

pub fn execute_skills(args: &SkillsArgs) -> Result<(), SkillsError> {
    let install_dir = match &args.dir {
        Some(d) => expand_tilde(d),
        None => get_default_install_dir(),
    };

    let skills_src = find_skills_src(&install_dir);
    let is_terminal = io::stdout().is_terminal();
    let colors = Colors::new(is_terminal);
    let mut stdout = io::stdout();

    if args.sync {
        cmd_sync(&skills_src, args.dry_run, &colors, &mut stdout)?;
        return Ok(());
    }

    if args.list {
        cmd_list(&skills_src, &colors, &mut stdout)?;
        return Ok(());
    }

    if args.prompt {
        cmd_prompt::<io::StdinLock, _>(&skills_src, args.dry_run, &colors, None, &mut stdout)?;
        return Ok(());
    }

    if args.no_skills {
        let (selected, _) = parse_skills_selection("none");
        apply_all_targets(&selected, &skills_src, args.dry_run, &colors, &mut stdout)?;
        return Ok(());
    }

    if let Some(ref skills_val) = args.skills {
        let (selected, warnings) = parse_skills_selection(skills_val);
        for warn in warnings {
            eprintln!("{}warning:{} {}", colors.yellow, colors.reset, warn);
        }
        apply_all_targets(&selected, &skills_src, args.dry_run, &colors, &mut stdout)?;
        return Ok(());
    }

    if !args.targets.is_empty() {
        if args.targets.len() == 1 {
            let first = args.targets[0].to_lowercase();
            match first.as_str() {
                "list" | "ls" => {
                    cmd_list(&skills_src, &colors, &mut stdout)?;
                    return Ok(());
                }
                "sync" | "update" => {
                    cmd_sync(&skills_src, args.dry_run, &colors, &mut stdout)?;
                    return Ok(());
                }
                "prompt" | "interactive" => {
                    cmd_prompt::<io::StdinLock, _>(
                        &skills_src,
                        args.dry_run,
                        &colors,
                        None,
                        &mut stdout,
                    )?;
                    return Ok(());
                }
                _ => {}
            }
        }

        let combined = args.targets.join(" ");
        let (selected, warnings) = parse_skills_selection(&combined);
        for warn in warnings {
            eprintln!("{}warning:{} {}", colors.yellow, colors.reset, warn);
        }
        apply_all_targets(&selected, &skills_src, args.dry_run, &colors, &mut stdout)?;
        return Ok(());
    }

    // Default action: if interactive prompt, else list
    if io::stdin().is_terminal() {
        cmd_prompt::<io::StdinLock, _>(&skills_src, args.dry_run, &colors, None, &mut stdout)?;
    } else {
        cmd_list(&skills_src, &colors, &mut stdout)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_args_parsing() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            command: TestCommands,
        }

        #[derive(clap::Subcommand)]
        enum TestCommands {
            Skills(SkillsArgs),
        }

        let cli = TestCli::parse_from(["gwt", "skills", "claude,gemini", "-n"]);
        match cli.command {
            TestCommands::Skills(args) => {
                assert!(args.dry_run);
                assert_eq!(args.targets, vec!["claude,gemini"]);
            }
        }
    }

    #[test]
    fn test_skills_error_exit_code() {
        let err = SkillsError::NotFound("test".to_string());
        assert_eq!(err.exit_code(), 47);

        let io_err = SkillsError::Io(io::Error::new(io::ErrorKind::Other, "test"));
        assert_eq!(io_err.exit_code(), 1);
    }
}
