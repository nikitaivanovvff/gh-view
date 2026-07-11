use super::dashboard::render_dashboard;
use super::detail::render_detail;
use super::theme;
use super::theme_picker::render_theme_picker;
use crate::app::{App, AppView};

pub(super) fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    theme::set_active_theme(app.active_theme_index());
    let area = frame.area();
    frame.render_widget(
        ratatui::widgets::Block::default().style(theme::background()),
        area,
    );

    match app.view {
        AppView::Dashboard => render_dashboard(frame, app),
        AppView::Detail => render_detail(frame, app),
    }

    if app.theme_picker_is_open() {
        render_theme_picker(frame, app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::MockGhClient;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn constrained_dashboard_render_does_not_mutate_selection_or_scroll() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.dashboard.selected = usize::MAX;
        app.dashboard.scroll = u16::MAX;
        let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();

        assert_eq!(app.dashboard.selected, usize::MAX);
        assert_eq!(app.dashboard.scroll, u16::MAX);
    }
}
