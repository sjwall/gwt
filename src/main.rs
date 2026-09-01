use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // Create a new worktree branch
    Add(AddArgs),
    // Switch to a worktree matching NAME
    Cd(CdArgs),
    // Switch to a worktree matching NAME and launch configured IDE
    #[command(alias = "s")]
    Switch(SwitchArgs),
    // List tracked worktrees
    #[command(alias = "ls")]
    List(ListArgs),
    // Track a git repository
    #[command(alias = "t")]
    Track(TrackArgs),
    // View or set configuration options
    Config(ConfigArgs),
    // Get or set configured IDE (defaults to nvim)
    Ide(IdeArgs),
}

#[derive(Args)]
struct AddArgs {
    // Worktree name, and optional --ide and --no-install flags
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Args)]
struct SwitchArgs {
    // Worktree name to match and optional --ide flag
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Args)]
struct CdArgs {
    // Worktree name to match
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Args)]
struct ListArgs {
    // Worktree name to match to limit the list to
    name: Option<String>,
}

#[derive(Args)]
struct TrackArgs {
    // Path to the git repository to track (defaults to current repository)
    path: Option<String>,
}

#[derive(Args)]
struct ConfigArgs {
    // Configuration action or arguments (e.g. get, set, unset, or key [value])
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Args)]
struct IdeArgs {
    // IDE command or name to configure (e.g. nvim, code, cursor, none)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    ide: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Add(args) => {
            if let Err(err) = gwt::commands::add::run(&args.args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Cd(args) => {
            if let Err(err) = gwt::commands::cd::run(&args.args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Switch(args) => {
            if let Err(err) = gwt::commands::switch::run(&args.args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::List(args) => {
            if let Err(err) = gwt::commands::list::list_and_print(args.name.as_deref()) {
                eprintln!("gwt: {err}");
                std::process::exit(1);
            }
        }
        Commands::Track(args) => {
            if let Err(err) = gwt::commands::track::track_and_print(args.path.as_deref()) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Config(args) => {
            if let Err(err) = gwt::commands::config::run_config(&args.args) {
                eprintln!("gwt: {err}");
                std::process::exit(err.exit_code());
            }
        }
        Commands::Ide(args) => {
            if let Err(err) = gwt::commands::ide::run(&args.ide) {
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
