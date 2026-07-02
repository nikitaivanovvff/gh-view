use anyhow::Result;
use clap::{Parser, Subcommand};

mod app;
mod github;
mod model;
mod ui;

#[derive(Debug, Parser)]
#[command(name = "gh-view")]
#[command(about = "A terminal view for GitHub pull requests")]
#[command(version)]
struct Cli {
    /// Use built-in mock pull request data instead of calling gh.
    #[arg(long, global = true)]
    mock: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Launch the pull request dashboard.
    Dashboard,
    /// Check local dependencies needed by gh-view.
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Dashboard) {
        Commands::Dashboard => ui::run(client(cli.mock)),
        Commands::Doctor => run_doctor(cli.mock),
    }
}

fn client(mock: bool) -> Box<dyn github::PullRequestSource> {
    if mock {
        Box::new(github::MockGhClient::new())
    } else {
        Box::new(github::GhClient::new())
    }
}

fn run_doctor(mock: bool) -> Result<()> {
    let client = client(mock);

    println!("gh-view doctor");
    println!("Rust CLI is installed and runnable.");

    if mock {
        println!("Data source: built-in mock data");
    }

    match client.status() {
        github::GhStatus::Ready { version } => {
            println!("GitHub CLI: {version}");
            println!("GitHub auth: configured");
        }
        github::GhStatus::Missing => {
            println!("GitHub CLI: not found on PATH; install gh before fetching PRs.");
        }
        github::GhStatus::Unauthenticated { message } => {
            println!("GitHub CLI: installed but not authenticated");
            println!("Run: gh auth login");
            if !message.is_empty() {
                println!("Details: {message}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn default_command_is_dashboard() {
        let cli = Cli::try_parse_from(["gh-view"]).unwrap();
        assert!(!cli.mock);
        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_global_mock_before_subcommand() {
        let cli = Cli::try_parse_from(["gh-view", "--mock", "doctor"]).unwrap();
        assert!(cli.mock);
        assert!(matches!(cli.command, Some(Commands::Doctor)));
    }

    #[test]
    fn parses_global_mock_after_subcommand() {
        let cli = Cli::try_parse_from(["gh-view", "dashboard", "--mock"]).unwrap();
        assert!(cli.mock);
        assert!(matches!(cli.command, Some(Commands::Dashboard)));
    }
}
