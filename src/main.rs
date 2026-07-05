use anyhow::Result;
use clap::{Parser, Subcommand};

mod app;
mod config;
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
    let config = config::Config::load()?;

    match cli.command.unwrap_or(Commands::Dashboard) {
        Commands::Dashboard => ui::run(client(cli.mock, &config)),
        Commands::Doctor => run_doctor(cli.mock, &config),
    }
}

fn client(mock: bool, config: &config::Config) -> Box<dyn github::PullRequestSource> {
    if mock {
        Box::new(github::MockGhClient::with_timeout(
            config.gh_timeout_seconds,
        ))
    } else {
        Box::new(github::GhClient::new(config.gh_timeout_seconds))
    }
}

fn run_doctor(mock: bool, config: &config::Config) -> Result<()> {
    let client = client(mock, config);

    println!("gh-view doctor");
    println!("Rust CLI is installed and runnable.");
    println!("gh command timeout: {}s", config.gh_timeout_seconds);

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
