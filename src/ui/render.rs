use super::dashboard::render_dashboard;
use super::detail::render_detail;
use super::theme;
use super::theme_picker::render_theme_picker;
use crate::app::{App, AppView};

pub(super) fn render(frame: &mut ratatui::Frame<'_>, app: &mut App) {
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
