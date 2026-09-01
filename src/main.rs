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
    // List tracked worktrees
    #[command(alias = "ls")]
    List(ListArgs),
}

#[derive(Args)]
struct ListArgs {
    // Worktree name to match to limit the list to
    name: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::List(args) => {
            if let Err(err) = gwt::commands::list::list_and_print(args.name.as_deref()) {
                eprintln!("gwt: {err}");
                std::process::exit(1);
            }
        }
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
