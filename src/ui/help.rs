use super::theme;
use crate::app::{App, AppView};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const DASHBOARD_SHORTCUTS: &[(&str, &str)] = &[
    ("j/k or up/down", "move selection"),
    ("1/2 or tab", "switch dashboard view"),
    ("f", "cycle review filter"),
    ("enter", "open selected PR"),
    ("/", "search loaded PRs"),
    ("t", "open theme picker"),
    ("c", "copy selected branch"),
    ("space or o", "toggle repository"),
    ("n/p or left/right", "change repository page"),
    ("b", "open selected PR in browser"),
    ("r", "refresh dashboard"),
    ("q or esc", "quit"),
];

const DETAIL_SHORTCUTS: &[(&str, &str)] = &[
    ("j/k or up/down", "scroll focused pane"),
    ("tab", "switch focused pane"),
    ("n/p or left/right", "change discussion item"),
    ("b", "open PR in browser"),
    ("r", "refresh PR detail"),
    ("q or esc", "back to dashboard"),
];

pub(super) fn render_help(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    let (title, shortcuts) = match app.view {
        AppView::Dashboard => ("Dashboard shortcuts", DASHBOARD_SHORTCUTS),
        AppView::Detail => ("PR detail shortcuts", DETAIL_SHORTCUTS),
    };
    let extra = usize::from(app.is_mock() && app.view == AppView::Dashboard);
    let width = 56.min(area.width);
    let height = (shortcuts.len() + extra + 6) as u16;
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let inner_width = popup.width.saturating_sub(2) as usize;
    let key_width = shortcuts
        .iter()
        .map(|(key, _)| key.len())
        .max()
        .unwrap_or_default()
        .max(6);
    let mut lines = vec![
        Line::styled(title, theme::accent().add_modifier(Modifier::BOLD)),
        rule(inner_width),
    ];
    lines.extend(shortcuts.iter().map(|(key, label)| {
        Line::from(vec![
            Span::styled(format!("{key:<key_width$}"), theme::muted_key()),
            Span::raw("  "),
            Span::styled(*label, theme::normal()),
        ])
    }));
    if extra > 0 {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<key_width$}", "f1"), theme::muted_key()),
            Span::raw("  "),
            Span::styled("open mock debug", theme::normal()),
        ]));
    }
    lines.push(rule(inner_width));
    lines.push(Line::from(vec![
        Span::styled("?/esc/q", theme::muted_key()),
        Span::styled(" close", theme::muted()),
    ]));

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme::focus_rule()),
            )
            .style(theme::background()),
        popup,
    );
}

fn rule(width: usize) -> Line<'static> {
    Line::from(Span::styled("─".repeat(width.max(1)), theme::rule()))
}
