use super::dashboard::{
    group_line, message_line, pr_line, reviewers_line, section_count, section_lines,
};
use super::detail::render_detail;
use super::text::rule_line;
use super::theme;
use crate::app::{App, AppView, Row};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub(super) fn render(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    if app.view == AppView::Detail {
        render_detail(frame, app);
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

    let mut header = vec![
        Span::styled("GH-VIEW", theme::accent().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            app.current_user
                .as_deref()
                .map(|user| format!("@{user}"))
                .unwrap_or_else(|| "@loading".to_owned()),
            theme::muted(),
        ),
    ];
    if app.dashboard_loading {
        header.push(Span::raw("  "));
        header.push(Span::styled("loading…", theme::muted()));
    }
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

    let footer = vec![
        rule_line(width),
        Line::from(vec![
            Span::styled("j/k", theme::muted_key()),
            Span::styled(" move   ", theme::muted()),
            Span::styled("enter", theme::muted_key()),
            Span::styled(" details   ", theme::muted()),
            Span::styled("o", theme::muted_key()),
            Span::styled(" toggle group   ", theme::muted()),
            Span::styled("/", theme::muted_key()),
            Span::styled(" search   ", theme::muted()),
            Span::styled("r", theme::muted_key()),
            Span::styled(" refresh   ", theme::muted()),
            Span::styled("q", theme::muted_key()),
            Span::styled(" quit", theme::muted()),
        ]),
    ];
    frame.render_widget(Paragraph::new(footer), chunks[1]);
}
