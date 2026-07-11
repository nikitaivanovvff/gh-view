use super::input::{InputOutcome, handle_event};
use super::render::render;
use crate::app::App;
use crate::config::Config;
use crate::github::PullRequestSource;
use anyhow::{Context, Result};
use crossterm::cursor::Show;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::{Duration, Instant};

pub fn run(client: Box<dyn PullRequestSource>, config: Config) -> Result<()> {
    let active_theme = super::theme::theme_index(&config.ui.theme);
    let mut app = App::with_active_theme(client, config, active_theme);
    app.refresh_async();

    let (mut terminal, mut guard) = setup_terminal()?;
    let result = run_app(&mut terminal, &mut app);
    match result {
        Ok(()) => guard.restore(),
        Err(error) => Err(error),
    }
}

fn setup_terminal() -> Result<(Terminal<CrosstermBackend<io::Stdout>>, TerminalGuard)> {
    let mut guard = TerminalGuard::default();

    enable_raw_mode().context("failed to enable raw mode")?;
    guard.raw_mode = true;

    execute!(io::stdout(), EnterAlternateScreen).context("failed to enter alternate screen")?;
    guard.alternate_screen = true;

    execute!(io::stdout(), EnableMouseCapture).context("failed to enable mouse capture")?;
    guard.mouse_capture = true;
    guard.restore_cursor = true;

    let terminal =
        Terminal::new(CrosstermBackend::new(io::stdout())).context("failed to create terminal")?;
    Ok((terminal, guard))
}

#[derive(Default)]
struct TerminalGuard {
    raw_mode: bool,
    alternate_screen: bool,
    mouse_capture: bool,
    restore_cursor: bool,
}

impl TerminalGuard {
    fn restore(&mut self) -> Result<()> {
        let mut first_error = None;

        if self.mouse_capture {
            match execute!(io::stdout(), DisableMouseCapture)
                .context("failed to disable mouse capture")
            {
                Ok(()) => self.mouse_capture = false,
                Err(error) => first_error = Some(error),
            }
        }
        if self.alternate_screen {
            match execute!(io::stdout(), LeaveAlternateScreen)
                .context("failed to leave alternate screen")
            {
                Ok(()) => self.alternate_screen = false,
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if self.restore_cursor {
            match execute!(io::stdout(), Show).context("failed to restore cursor") {
                Ok(()) => self.restore_cursor = false,
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if self.raw_mode {
            match disable_raw_mode().context("failed to disable raw mode") {
                Ok(()) => self.raw_mode = false,
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }

        first_error.map_or(Ok(()), Err)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut needs_draw = true;
    let loading_tick = Duration::from_millis(200);
    let mut last_loading_tick = Instant::now();

    loop {
        if app.poll_background() {
            needs_draw = true;
        }

        if app.is_loading() && last_loading_tick.elapsed() >= loading_tick {
            app.advance_loading_frame();
            last_loading_tick = Instant::now();
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
