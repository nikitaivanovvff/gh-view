use super::dashboard::render_dashboard;
use super::detail::render_detail;
use crate::app::{App, AppView};

pub(super) fn render(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    match app.view {
        AppView::Dashboard => render_dashboard(frame, app),
        AppView::Detail => render_detail(frame, app),
    }
}
