use clap::{Parser, Subcommand};
use gwt::commands::add::AddArgs;
use gwt::commands::agent::AgentArgs;
use gwt::commands::cd::CdArgs;
use gwt::commands::config::ConfigArgs;
use gwt::commands::ide::IdeArgs;
use gwt::commands::list::ListArgs;
use gwt::commands::main::{MainArgs, MainIdeArgs};
use gwt::commands::pull::PullArgs;
use gwt::commands::remove::RemoveArgs;
use gwt::commands::skills::SkillsArgs;
use gwt::commands::switch::SwitchArgs;
use gwt::commands::track::TrackArgs;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new worktree branch
    Add(AddArgs),
    /// Switch to a worktree matching NAME
    Cd(CdArgs),
    /// Fetch origin/NAME, create tracking worktree, cd, yarn, launch IDE
    #[command(alias = "p")]
    Pull(PullArgs),
    /// Switch to a worktree matching NAME and launch configured IDE
    #[command(alias = "s")]
    Switch(SwitchArgs),
    /// Switch to a worktree matching NAME and launch configured agent
    #[command(alias = "a")]
    Agent(AgentArgs),
    /// List tracked worktrees
    #[command(alias = "ls")]
    List(ListArgs),
    /// Remove a worktree
    #[command(alias = "rm")]
    Remove(RemoveArgs),
    /// Manage global agent skill symlinks
    Skills(SkillsArgs),
    /// Track a git repository
    #[command(alias = "t")]
    Track(TrackArgs),
    /// Switch directory to the main repository
    #[command(alias = "m")]
    Main(MainArgs),
    /// Switch directory to the main repository and launch configured IDE
    #[command(name = "M", alias = "Main")]
    MainIde(MainIdeArgs),
    /// View or set configuration options
    Config(ConfigArgs),
    /// Get or set configured IDE (defaults to nvim)
    Ide(IdeArgs),
}

/// Preprocesses raw CLI arguments to support default subcommand omission (`gwt <branch>`),
/// rewriting to `gwt add <branch>` when no known subcommand or top-level flag is specified.
pub fn preprocess_cli_args<I, T>(raw_args: I) -> Vec<String>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let mut args: Vec<String> = raw_args.into_iter().map(Into::into).collect();
    if args.is_empty() {
        return vec!["gwt".into(), "add".into()];
    }

    // If no arguments were passed after binary name, rewrite to `add`
    if args.len() == 1 {
        args.push("add".into());
        return args;
    }

    let first_arg = &args[1];

    // Top-level flags for help and version should not be rewritten
    if first_arg == "--help"
        || first_arg == "-h"
        || first_arg == "help"
        || first_arg == "--version"
        || first_arg == "-V"
    {
        return args;
    }

    // Known subcommands and aliases
    let is_subcommand = matches!(
        first_arg.as_str(),
        "add"
            | "agent"
            | "a"
            | "cd"
            | "config"
            | "ide"
            | "list"
            | "ls"
            | "main"
            | "m"
            | "M"
            | "Main"
            | "migrate"
            | "pull"
            | "p"
            | "remove"
            | "rm"
            | "skills"
            | "switch"
            | "s"
            | "track"
            | "t"
            | "upgrade"
    );

    if !is_subcommand {
        args.insert(1, "add".into());
    }

    args
}

fn main() {
    let args = preprocess_cli_args(std::env::args());
    let cli = Cli::parse_from(args);

    match &cli.command {
        Commands::Add(args) => {
            if let Err(err) = gwt::commands::add::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Cd(args) => {
            if let Err(err) = gwt::commands::cd::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Pull(args) => {
            if let Err(err) = gwt::commands::pull::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Switch(args) => {
            if let Err(err) = gwt::commands::switch::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Agent(args) => {
            if let Err(err) = gwt::commands::agent::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::List(args) => {
            if let Err(err) = gwt::commands::list::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Remove(args) => {
            if let Err(err) = gwt::commands::remove::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Skills(args) => {
            if let Err(err) = gwt::commands::skills::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Track(args) => {
            if let Err(err) = gwt::commands::track::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Main(args) => {
            if let Err(err) = gwt::commands::main::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::MainIde(args) => {
            if let Err(err) = gwt::commands::main::run_ide_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Config(args) => {
            if let Err(err) = gwt::commands::config::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Ide(args) => {
            if let Err(err) = gwt::commands::ide::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}

#[test]
fn test_cli_track_parsing() {
    let cli = Cli::try_parse_from(["gwt", "track"]).unwrap();
    match cli.command {
        Commands::Track(args) => assert!(args.args.is_empty()),
        _ => panic!("expected Track command"),
    }

    let cli = Cli::try_parse_from(["gwt", "track", "my_repo"]).unwrap();
    match cli.command {
        Commands::Track(args) => assert_eq!(args.args, vec!["my_repo"]),
        _ => panic!("expected Track command"),
    }

    let cli = Cli::try_parse_from(["gwt", "track", "arg1", "arg2"]).unwrap();
    match cli.command {
        Commands::Track(args) => assert_eq!(args.args, vec!["arg1", "arg2"]),
        _ => panic!("expected Track command"),
    }
}

#[test]
fn test_cli_skills_parsing() {
    let cli = Cli::try_parse_from(["gwt", "skills"]).unwrap();
    match cli.command {
        Commands::Skills(args) => {
            assert!(args.targets.is_empty());
            assert!(!args.dry_run);
        }
        _ => panic!("expected Skills command"),
    }

    let cli = Cli::try_parse_from(["gwt", "skills", "claude,gemini", "-n"]).unwrap();
    match cli.command {
        Commands::Skills(args) => {
            assert_eq!(args.targets, vec!["claude,gemini"]);
            assert!(args.dry_run);
        }
        _ => panic!("expected Skills command"),
    }

    let cli = Cli::try_parse_from(["gwt", "skills", "--sync"]).unwrap();
    match cli.command {
        Commands::Skills(args) => {
            assert!(args.sync);
        }
        _ => panic!("expected Skills command"),
    }
}

#[test]
fn test_cli_main_parsing() {
    let cli = Cli::try_parse_from(["gwt", "main"]).unwrap();
    match cli.command {
        Commands::Main(args) => assert!(args.args.is_empty()),
        _ => panic!("expected Main command"),
    }

    let cli = Cli::try_parse_from(["gwt", "main", "my_repo"]).unwrap();
    match cli.command {
        Commands::Main(args) => assert_eq!(args.args, vec!["my_repo"]),
        _ => panic!("expected Main command"),
    }

    let cli = Cli::try_parse_from(["gwt", "m", "my_repo"]).unwrap();
    match cli.command {
        Commands::Main(args) => assert_eq!(args.args, vec!["my_repo"]),
        _ => panic!("expected Main command"),
    }

    let cli = Cli::try_parse_from(["gwt", "main", "arg1", "arg2"]).unwrap();
    match cli.command {
        Commands::Main(args) => assert_eq!(args.args, vec!["arg1", "arg2"]),
        _ => panic!("expected Main command"),
    }
}

#[test]
fn test_cli_main_ide_parsing() {
    let cli = Cli::try_parse_from(["gwt", "M"]).unwrap();
    match cli.command {
        Commands::MainIde(args) => assert!(args.args.is_empty()),
        _ => panic!("expected MainIde command"),
    }

    let cli = Cli::try_parse_from(["gwt", "M", "my_repo"]).unwrap();
    match cli.command {
        Commands::MainIde(args) => assert_eq!(args.args, vec!["my_repo"]),
        _ => panic!("expected MainIde command"),
    }

    let cli = Cli::try_parse_from(["gwt", "Main", "my_repo"]).unwrap();
    match cli.command {
        Commands::MainIde(args) => assert_eq!(args.args, vec!["my_repo"]),
        _ => panic!("expected MainIde command"),
    }

    let cli = Cli::try_parse_from(["gwt", "M", "--ide", "code", "my_repo"]).unwrap();
    match cli.command {
        Commands::MainIde(args) => assert_eq!(args.args, vec!["--ide", "code", "my_repo"]),
        _ => panic!("expected MainIde command"),
    }
}

#[test]
fn test_cli_agent_parsing() {
    let cli = Cli::try_parse_from(["gwt", "agent"]).unwrap();
    match cli.command {
        Commands::Agent(args) => assert!(args.args.is_empty()),
        _ => panic!("expected Agent command"),
    }

    let cli = Cli::try_parse_from(["gwt", "agent", "my_wt"]).unwrap();
    match cli.command {
        Commands::Agent(args) => assert_eq!(args.args, vec!["my_wt"]),
        _ => panic!("expected Agent command"),
    }

    // Verify alias 'a' maps to Agent, NOT Add
    let cli = Cli::try_parse_from(["gwt", "a", "my_wt"]).unwrap();
    match cli.command {
        Commands::Agent(args) => assert_eq!(args.args, vec!["my_wt"]),
        _ => panic!("expected Agent command for alias 'a'"),
    }

    let cli = Cli::try_parse_from(["gwt", "agent", "--agent", "opencode", "my_wt"]).unwrap();
    match cli.command {
        Commands::Agent(args) => assert_eq!(args.args, vec!["--agent", "opencode", "my_wt"]),
        _ => panic!("expected Agent command"),
    }
}

#[test]
fn test_cli_add_parsing() {
    let cli = Cli::try_parse_from(["gwt", "add", "feat-1"]).unwrap();
    match cli.command {
        Commands::Add(args) => assert_eq!(args.args, vec!["feat-1"]),
        _ => panic!("expected Add command"),
    }

    let cli = Cli::try_parse_from(["gwt", "add", "--agent", "feat-2"]).unwrap();
    match cli.command {
        Commands::Add(args) => assert_eq!(args.args, vec!["--agent", "feat-2"]),
        _ => panic!("expected Add command"),
    }

    let cli = Cli::try_parse_from(["gwt", "add", "-a", "feat-3"]).unwrap();
    match cli.command {
        Commands::Add(args) => assert_eq!(args.args, vec!["-a", "feat-3"]),
        _ => panic!("expected Add command"),
    }
}

#[test]
fn test_preprocess_cli_args() {
    // Default omission: branch directly after binary
    assert_eq!(
        preprocess_cli_args(["gwt", "my-feature"]),
        vec!["gwt", "add", "my-feature"]
    );

    // Default omission with flags
    assert_eq!(
        preprocess_cli_args(["gwt", "--ide", "code", "my-feature"]),
        vec!["gwt", "add", "--ide", "code", "my-feature"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "--agent", "my-feature"]),
        vec!["gwt", "add", "--agent", "my-feature"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "-a", "my-feature"]),
        vec!["gwt", "add", "-a", "my-feature"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "--no-install", "my-feature"]),
        vec!["gwt", "add", "--no-install", "my-feature"]
    );

    // Empty arguments rewrites to add
    assert_eq!(preprocess_cli_args(["gwt"]), vec!["gwt", "add"]);

    // Explicit subcommands should not be rewritten
    assert_eq!(
        preprocess_cli_args(["gwt", "add", "my-feature"]),
        vec!["gwt", "add", "my-feature"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "a", "my-feature"]),
        vec!["gwt", "a", "my-feature"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "agent", "my-feature"]),
        vec!["gwt", "agent", "my-feature"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "switch", "my-feature"]),
        vec!["gwt", "switch", "my-feature"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "s", "my-feature"]),
        vec!["gwt", "s", "my-feature"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "main"]),
        vec!["gwt", "main"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "m"]),
        vec!["gwt", "m"]
    );
    assert_eq!(
        preprocess_cli_args(["gwt", "M"]),
        vec!["gwt", "M"]
    );

    // Help and version flags should not be rewritten
    assert_eq!(preprocess_cli_args(["gwt", "--help"]), vec!["gwt", "--help"]);
    assert_eq!(preprocess_cli_args(["gwt", "-h"]), vec!["gwt", "-h"]);
    assert_eq!(preprocess_cli_args(["gwt", "help"]), vec!["gwt", "help"]);
    assert_eq!(preprocess_cli_args(["gwt", "--version"]), vec!["gwt", "--version"]);
    assert_eq!(preprocess_cli_args(["gwt", "-V"]), vec!["gwt", "-V"]);
}

#[test]
fn test_cli_default_invocation() {
    let args = preprocess_cli_args(["gwt", "my-feature-branch"]);
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Add(add_args) => {
            assert_eq!(add_args.args, vec!["my-feature-branch"]);
        }
        _ => panic!("expected Add command from default invocation"),
    }
}

#[test]
fn test_cli_list_parsing() {
    let cli = Cli::try_parse_from(["gwt", "list"]).unwrap();
    match cli.command {
        Commands::List(args) => assert!(args.args.is_empty()),
        _ => panic!("expected List command"),
    }

    let cli = Cli::try_parse_from(["gwt", "ls"]).unwrap();
    match cli.command {
        Commands::List(args) => assert!(args.args.is_empty()),
        _ => panic!("expected List command"),
    }

    let cli = Cli::try_parse_from(["gwt", "ls", "my-repo"]).unwrap();
    match cli.command {
        Commands::List(args) => assert_eq!(args.args, vec!["my-repo"]),
        _ => panic!("expected List command"),
    }

    let cli = Cli::try_parse_from(["gwt", "ls", "--porcelain"]).unwrap();
    match cli.command {
        Commands::List(args) => assert_eq!(args.args, vec!["--porcelain"]),
        _ => panic!("expected List command"),
    }

    let cli = Cli::try_parse_from(["gwt", "ls", "-v", "my-repo"]).unwrap();
    match cli.command {
        Commands::List(args) => assert_eq!(args.args, vec!["-v", "my-repo"]),
        _ => panic!("expected List command"),
    }

    let cli = Cli::try_parse_from(["gwt", "ls", "my-repo", "--porcelain"]).unwrap();
    match cli.command {
        Commands::List(args) => assert_eq!(args.args, vec!["my-repo", "--porcelain"]),
        _ => panic!("expected List command"),
    }

    let cli = Cli::try_parse_from(["gwt", "ls", "repo1", "repo2"]).unwrap();
    match cli.command {
        Commands::List(args) => assert_eq!(args.args, vec!["repo1", "repo2"]),
        _ => panic!("expected List command"),
    }
}


