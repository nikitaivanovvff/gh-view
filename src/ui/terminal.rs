use super::input::{InputOutcome, handle_event};
use super::render::render;
use crate::app::App;
use crate::github::PullRequestSource;
use anyhow::{Context, Result};
use crossterm::event;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

pub fn run(client: Box<dyn PullRequestSource>) -> Result<()> {
    let mut app = App::new(client);
    app.refresh_async();

    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, &mut app);
    restore_terminal()?;

    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().context("failed to enable raw mode")?;
    execute!(io::stdout(), EnterAlternateScreen).context("failed to enter alternate screen")?;
    Terminal::new(CrosstermBackend::new(io::stdout())).context("failed to create terminal")
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(io::stdout(), LeaveAlternateScreen).context("failed to leave alternate screen")
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut needs_draw = true;

    loop {
        if app.poll_background() {
            needs_draw = true;
        }

        if needs_draw {
            terminal.draw(|frame| render(frame, app))?;
            needs_draw = false;
        }

        if event::poll(Duration::from_millis(200))? {
            match handle_event(event::read()?, app)? {
                InputOutcome::Continue(changed) => needs_draw |= changed,
                InputOutcome::Quit => return Ok(()),
            }
        }
    }
}
