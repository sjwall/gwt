use clap::{Parser, Subcommand};
use gwt::commands::add::AddArgs;
use gwt::commands::cd::CdArgs;
use gwt::commands::config::ConfigArgs;
use gwt::commands::ide::IdeArgs;
use gwt::commands::list::ListArgs;
use gwt::commands::pull::PullArgs;
use gwt::commands::remove::RemoveArgs;
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
    #[command(alias = "a")]
    Add(AddArgs),
    /// Switch to a worktree matching NAME
    Cd(CdArgs),
    /// Fetch origin/NAME, create tracking worktree, cd, yarn, launch IDE
    #[command(alias = "p")]
    Pull(PullArgs),
    /// Switch to a worktree matching NAME and launch configured IDE
    #[command(alias = "s")]
    Switch(SwitchArgs),
    /// List tracked worktrees
    #[command(alias = "ls")]
    List(ListArgs),
    /// Remove a worktree
    #[command(alias = "rm")]
    Remove(RemoveArgs),
    /// Track a git repository
    #[command(alias = "t")]
    Track(TrackArgs),
    /// View or set configuration options
    Config(ConfigArgs),
    /// Get or set configured IDE (defaults to nvim)
    Ide(IdeArgs),
}

fn main() {
    let cli = Cli::parse();

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
        Commands::List(args) => {
            if let Err(err) = gwt::commands::list::run_args(args) {
                eprintln!("gwt: {err}");
                std::process::exit(1);
            }
        }
        Commands::Remove(args) => {
            if let Err(err) = gwt::commands::remove::run_args(args) {
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
