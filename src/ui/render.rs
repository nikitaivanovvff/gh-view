use super::dashboard::{
    group_line, message_line, pr_line, reviewers_line, section_count, section_lines,
};
use super::detail::render_detail;
use super::text::{loading_dots, rule_line};
use super::theme;
use crate::app::{App, AppView, DashboardErrorLine, DashboardErrorPage, Row};
use ratatui::layout::Alignment;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub(super) fn render(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    if app.view == AppView::Detail {
        render_detail(frame, app);
        return;
    }

    if app.show_dashboard_loading_screen() {
        render_dashboard_loading(frame, app.loading_frame);
        return;
    }

    if let Some(error_page) = app.dashboard_error_page() {
        render_dashboard_error(frame, &error_page);
        return;
    }

    app.clamp_selection();

    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);

    let rows = app.rows();
    let width = chunks[0].width as usize;
    let mut lines = Vec::new();

    let header = vec![
        Span::styled("GH-VIEW", theme::accent().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            app.current_user
                .as_deref()
                .map(|user| format!("@{user}"))
                .unwrap_or_else(|| "@unknown".to_owned()),
            theme::muted(),
        ),
    ];
    lines.push(Line::from(header));
    lines.push(rule_line(width));
    for (index, row) in rows.iter().enumerate() {
        match row {
            Row::Section(title) => {
                if *title == "Awaiting Review" {
                    lines.push(Line::raw(""));
                }
                lines.extend(section_lines(title, section_count(&rows, index), width));
            }
            Row::Group {
                repo, count, open, ..
            } => {
                lines.push(group_line(index == app.selected, repo, *count, *open));
            }
            Row::Pr(pr) => {
                lines.push(pr_line(index == app.selected, pr, width));
                lines.push(reviewers_line(pr));
            }
            Row::Message(message) => {
                lines.push(message_line(index == app.selected, message));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines).style(theme::normal()), chunks[0]);

    let mut footer_line = vec![
        Span::styled("j/k", theme::muted_key()),
        Span::styled(" move   ", theme::muted()),
        Span::styled("enter", theme::muted_key()),
        Span::styled(" details   ", theme::muted()),
        Span::styled("o", theme::muted_key()),
        Span::styled(" toggle group   ", theme::muted()),
        Span::styled("r", theme::muted_key()),
        Span::styled(" refresh   ", theme::muted()),
        Span::styled("q", theme::muted_key()),
        Span::styled(" quit", theme::muted()),
    ];
    if app.is_mock() {
        let mode = match app.mock_error_mode() {
            Some(crate::github::MockErrorMode::GitHubDown) => "down",
            Some(crate::github::MockErrorMode::Timeout) => "timeout",
            Some(crate::github::MockErrorMode::Generic) => "error",
            Some(crate::github::MockErrorMode::Auth) => "auth",
            None => "ok",
        };
        footer_line.extend([
            Span::styled("   mock ", theme::muted()),
            Span::styled(format!("[{mode}] "), theme::accent()),
            Span::styled("0", theme::muted_key()),
            Span::styled(" ok ", theme::muted()),
            Span::styled("1", theme::muted_key()),
            Span::styled(" down ", theme::muted()),
            Span::styled("2", theme::muted_key()),
            Span::styled(" timeout ", theme::muted()),
            Span::styled("3", theme::muted_key()),
            Span::styled(" error ", theme::muted()),
            Span::styled("4", theme::muted_key()),
            Span::styled(" auth", theme::muted()),
        ]);
    }
    let footer = vec![rule_line(width), Line::from(footer_line)];
    frame.render_widget(Paragraph::new(footer), chunks[1]);
}

fn render_dashboard_error(frame: &mut ratatui::Frame<'_>, page: &DashboardErrorPage) {
    let area = frame.area();
    let mut lines = Vec::new();
    for art_line in &page.art {
        lines.push(Line::styled(art_line.clone(), theme::muted()));
    }
    if !page.art.is_empty() && !page.lines.is_empty() {
        lines.push(Line::raw(""));
    }
    for line in &page.lines {
        lines.push(match line {
            DashboardErrorLine::Text(text) => Line::styled(text.clone(), theme::muted()),
            DashboardErrorLine::StatusPage => Line::from(vec![
                Span::styled("Check ", theme::muted()),
                Span::styled(
                    "https://www.githubstatus.com/",
                    theme::accent().add_modifier(Modifier::BOLD),
                ),
                Span::styled(" and press r to retry.", theme::muted()),
            ]),
        });
    }

    let height = lines.len().min(area.height as usize) as u16;
    let top = area.y + area.height.saturating_sub(height) / 2;
    let centered = Rect {
        x: area.x,
        y: top,
        width: area.width,
        height,
    };
    frame.render_widget(
        Paragraph::new(lines)
            .style(theme::normal())
            .alignment(Alignment::Center),
        centered,
    );
}

fn render_dashboard_loading(frame: &mut ratatui::Frame<'_>, loading_frame: usize) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let line = Line::from(vec![
        Span::styled("Loading PRs", theme::muted()),
        Span::styled(loading_dots(loading_frame), theme::accent()),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .style(theme::normal())
            .alignment(Alignment::Center),
        chunks[1],
    );
}
