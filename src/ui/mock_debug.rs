use super::theme;
use crate::app::App;
use crate::github::MockErrorMode;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const MODES: [(&str, &str, Option<MockErrorMode>); 5] = [
    ("0", "Ready", None),
    ("5", "GitHub outage", Some(MockErrorMode::GitHubDown)),
    ("6", "Request timeout", Some(MockErrorMode::Timeout)),
    ("7", "Generic error", Some(MockErrorMode::Generic)),
    ("8", "Authentication", Some(MockErrorMode::Auth)),
];

pub(super) fn render_mock_debug(frame: &mut ratatui::Frame<'_>, app: &App) {
    let Some(popup) = popup_area(frame.area()) else {
        return;
    };
    let width = popup.width.saturating_sub(4) as usize;
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Mock debug", theme::accent().add_modifier(Modifier::BOLD)),
            Span::styled("  dashboard states", theme::muted()),
        ]),
        rule(width),
    ];

    for (key, label, mode) in MODES {
        let active = app.mock_error_mode() == mode;
        let marker = if active { "▸" } else { " " };
        lines.push(Line::from(vec![
            Span::styled(marker, theme::accent().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(format!("{key:<3}"), theme::muted_key()),
            Span::styled(label, theme::normal()),
        ]));
    }

    lines.extend([
        rule(width),
        Line::from(vec![
            Span::styled("0/5-8", theme::muted_key()),
            Span::styled(" load state  ", theme::muted()),
            Span::styled("f1/esc", theme::muted_key()),
            Span::styled(" close", theme::muted()),
        ]),
    ]);

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

fn popup_area(area: Rect) -> Option<Rect> {
    if area.width < 24 || area.height < 10 {
        return None;
    }
    let width = 44.min(area.width);
    let height = 11.min(area.height);
    Some(Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    })
}

fn rule(width: usize) -> Line<'static> {
    Line::from(Span::styled("─".repeat(width.max(1)), theme::rule()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_fits_regular_and_rejects_tiny_terminals() {
        assert_eq!(popup_area(Rect::new(0, 0, 100, 30)).unwrap().width, 44);
        assert_eq!(popup_area(Rect::new(0, 0, 20, 8)), None);
    }
}
