use super::dashboard::render_dashboard;
use super::detail::render_detail;
use super::layout::{
    DASHBOARD_MIN_SIZE, DETAIL_MIN_SIZE, MOCK_DEBUG_MIN_SIZE, MouseLayout, SEARCH_MIN_SIZE,
    THEME_PICKER_MIN_SIZE,
};
use super::mock_debug::render_mock_debug;
use super::theme;
use super::theme_picker::render_theme_picker;
use crate::app::{App, AppView};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub(super) fn render(frame: &mut ratatui::Frame<'_>, app: &App) -> MouseLayout {
    let mut mouse_layout = MouseLayout::default();
    theme::set_active_theme(app.active_theme_index());
    let area = frame.area();
    frame.render_widget(
        ratatui::widgets::Block::default().style(theme::background()),
        area,
    );

    let (surface, minimum) = if app.theme_picker_is_open() {
        ("theme picker", THEME_PICKER_MIN_SIZE)
    } else if app.mock_debug_is_open() {
        ("mock debug", MOCK_DEBUG_MIN_SIZE)
    } else if app.search_is_open() {
        ("search", SEARCH_MIN_SIZE)
    } else {
        match app.view {
            AppView::Dashboard => ("dashboard", DASHBOARD_MIN_SIZE),
            AppView::Detail => ("PR detail", DETAIL_MIN_SIZE),
        }
    };
    if area.width < minimum.0 || area.height < minimum.1 {
        render_terminal_too_small(frame, surface, minimum);
        return MouseLayout::default();
    }

    match app.view {
        AppView::Dashboard => render_dashboard(frame, app, &mut mouse_layout),
        AppView::Detail => render_detail(frame, app, &mut mouse_layout),
    }

    if app.theme_picker_is_open() {
        render_theme_picker(frame, app, &mut mouse_layout);
    } else if app.mock_debug_is_open() {
        render_mock_debug(frame, app);
    }

    mouse_layout
}

fn render_terminal_too_small(frame: &mut ratatui::Frame<'_>, surface: &str, minimum: (u16, u16)) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Terminal too small",
                theme::danger().add_modifier(Modifier::BOLD),
            )),
            Line::from(format!(
                "Need {}x{} for {surface}; current {}x{}.",
                minimum.0, minimum.1, area.width, area.height
            )),
        ])
        .style(theme::normal())
        .alignment(Alignment::Center),
        rows[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{DashboardSection, DetailPane, DetailStatus, ReviewScope};
    use crate::github::MockGhClient;
    use ratatui::{Terminal, backend::TestBackend, layout::Position};
    use std::time::{Duration, Instant};

    fn draw_layout(app: &App, width: u16, height: u16) -> MouseLayout {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut layout = MouseLayout::default();
        terminal.draw(|frame| layout = render(frame, app)).unwrap();
        layout
    }

    fn draw_text(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| {
                render(frame, app);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn constrained_dashboard_render_does_not_mutate_selection_or_scroll() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.dashboard.selected = usize::MAX;
        app.dashboard.scroll = u16::MAX;
        let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();

        terminal
            .draw(|frame| {
                render(frame, &app);
            })
            .unwrap();

        assert_eq!(app.dashboard.selected, usize::MAX);
        assert_eq!(app.dashboard.scroll, u16::MAX);
    }

    #[test]
    fn dashboard_layout_tracks_rendered_rows_and_excludes_chrome() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        let layout = draw_layout(&app, 100, 30);

        assert_eq!(layout.target_at(Position::new(4, 0)), None);
        assert_eq!(
            layout.target_at(Position::new(4, 2)),
            Some(super::super::layout::MouseTarget::DashboardSection(
                DashboardSection::MyPrs
            ))
        );
        assert_eq!(
            layout.target_at(Position::new(4, 4)),
            Some(super::super::layout::MouseTarget::DashboardRow(1))
        );
        for row in 5..=7 {
            assert_eq!(
                layout.target_at(Position::new(4, row)),
                Some(super::super::layout::MouseTarget::DashboardRow(2))
            );
        }
        assert_eq!(layout.target_at(Position::new(4, 28)), None);

        app.dashboard.scroll = 6;
        let layout = draw_layout(&app, 100, 10);
        assert_eq!(
            layout.target_at(Position::new(4, 0)),
            Some(super::super::layout::MouseTarget::DashboardRow(2))
        );
        assert_eq!(
            layout.target_at(Position::new(4, 1)),
            Some(super::super::layout::MouseTarget::DashboardRow(2))
        );
        assert_ne!(
            layout.target_at(Position::new(4, 2)),
            Some(super::super::layout::MouseTarget::DashboardRow(2))
        );
    }

    #[test]
    fn dashboard_labels_use_exact_ranges() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        let first_label_width = format!(
            "1 {} [{}]",
            DashboardSection::MyPrs.title().to_ascii_uppercase(),
            app.dashboard.section_pr_count(DashboardSection::MyPrs)
        )
        .chars()
        .count() as u16;

        let layout = draw_layout(&app, 100, 30);

        assert_eq!(
            layout.target_at(Position::new(0, 2)),
            Some(super::super::layout::MouseTarget::DashboardSection(
                DashboardSection::MyPrs
            ))
        );
        assert_eq!(layout.target_at(Position::new(first_label_width, 2)), None);
        assert_eq!(
            layout.target_at(Position::new(first_label_width + 4, 2)),
            Some(super::super::layout::MouseTarget::DashboardSection(
                DashboardSection::AwaitingReview
            ))
        );

        app.show_dashboard_section(DashboardSection::AwaitingReview);
        let layout = draw_layout(&app, 100, 30);
        assert_eq!(
            layout.target_at(Position::new(75, 2)),
            Some(super::super::layout::MouseTarget::ReviewScope(
                ReviewScope::All
            ))
        );

        let narrow_layout = draw_layout(&app, 60, 30);
        assert_eq!(
            narrow_layout.target_at(Position::new(59, 2)),
            Some(super::super::layout::MouseTarget::ReviewScope(
                ReviewScope::All
            ))
        );
        assert_eq!(narrow_layout.target_at(Position::new(52, 2)), None);
    }

    #[test]
    fn narrow_dashboard_keeps_identity_ci_filter_and_essential_footer_controls() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        app.show_dashboard_section(DashboardSection::AwaitingReview);

        let text = draw_text(&app, 60, 20);

        assert!(text.contains("REVIEW REQUESTS"));
        assert!(text.contains("all [7]"));
        assert!(text.contains("#"));
        assert!(text.contains("ci"));
        assert!(text.contains("q quit"));
        assert!(text.contains("j/k move"));
    }

    #[test]
    fn loaded_dashboard_stays_visible_while_refreshing() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        app.dashboard.loading = true;

        let text = draw_text(&app, 80, 20);

        assert!(text.contains("Refreshing PRs"));
        assert!(text.contains("#"));
        assert!(text.contains("MY PRS"));
    }

    #[test]
    fn search_overlay_keeps_help_inside_a_standard_popup() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        app.open_search();

        let text = draw_text(&app, 100, 20);

        assert!(text.contains("Search PRs"));
        assert!(text.contains("enter open"));
        assert!(text.contains("esc close"));
    }

    #[test]
    fn representative_detail_layout_keeps_identity_and_essential_controls() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        app.next();
        app.next();
        app.open_selected_detail();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            app.poll_background();
            if app.detail.detail_status != DetailStatus::Loading {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let text = draw_text(&app, 80, 24);

        assert!(text.contains("GH-VIEW"));
        assert!(text.contains("#"));
        assert!(text.contains("DESCRIPTION"));
        assert!(text.contains("esc/q back"));
        assert!(text.contains("b open PR"));
        let discussion_row = text
            .lines()
            .position(|line| line.contains("DISCUSSION"))
            .unwrap();
        let code_row = text
            .lines()
            .position(|line| line.contains("CODE CONTEXT"))
            .unwrap();
        assert!(discussion_row < code_row);

        let wide = draw_text(&app, 120, 24);
        let discussion_row = wide
            .lines()
            .position(|line| line.contains("DISCUSSION"))
            .unwrap();
        let code_row = wide
            .lines()
            .position(|line| line.contains("CODE CONTEXT"))
            .unwrap();
        assert_eq!(discussion_row, code_row);
    }

    #[test]
    fn detail_and_theme_layouts_follow_rendered_areas_and_overlay_order() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.view = AppView::Detail;
        let layout = draw_layout(&app, 100, 30);
        assert_eq!(
            layout.target_at(Position::new(5, 5)),
            Some(super::super::layout::MouseTarget::DetailPane(
                DetailPane::Description
            ))
        );
        assert_eq!(layout.target_at(Position::new(5, 29)), None);

        app.open_theme_picker();
        let layout = draw_layout(&app, 100, 30);
        let popup =
            super::super::theme_picker::picker_area(ratatui::layout::Rect::new(0, 0, 100, 30))
                .unwrap();
        assert_eq!(
            layout.target_at(Position::new(popup.x, popup.y + 3)),
            Some(super::super::layout::MouseTarget::DetailPane(
                DetailPane::Description
            ))
        );
        assert_eq!(
            layout.target_at(Position::new(popup.x + 1, popup.y + 3)),
            Some(super::super::layout::MouseTarget::Theme(0))
        );
        assert_eq!(
            layout.target_at(Position::new(popup.x + 1, popup.y + 2)),
            Some(super::super::layout::MouseTarget::DetailPane(
                DetailPane::Description
            ))
        );
    }

    #[test]
    fn loading_dashboard_and_tiny_terminal_have_no_invalid_targets() {
        let app = App::with_default_config(Box::new(MockGhClient::new()));
        let layout = draw_layout(&app, 10, 2);

        assert_eq!(layout.target_at(Position::new(0, 0)), None);
        assert_eq!(layout.target_at(Position::new(9, 1)), None);
        assert!(draw_text(&app, 39, 10).contains("Need 40x10 for dashboard"));
        assert!(draw_text(&app, 40, 9).contains("Terminal too small"));
        assert!(!draw_text(&app, 40, 10).contains("Terminal too small"));
    }

    #[test]
    fn tiny_terminal_reports_active_overlay_requirement() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.open_search();
        assert!(draw_text(&app, 40, 8).contains("Need 40x9 for search"));

        app.close_search();
        app.open_theme_picker();
        assert!(draw_text(&app, 40, 14).contains("Need 40x15 for theme picker"));
        let layout = draw_layout(&app, 40, 14);
        assert_eq!(layout.target_at(Position::new(0, 0)), None);
        assert_eq!(layout.target_at(Position::new(39, 13)), None);

        app.cancel_theme_picker();
        app.view = AppView::Detail;
        assert!(draw_text(&app, 40, 23).contains("Need 40x24 for PR detail"));
    }
}
