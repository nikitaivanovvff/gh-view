use super::layout::{MouseLayout, MouseTarget};
use super::text::truncate;
use super::theme;
use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub(super) fn render_theme_picker(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    mouse_layout: &mut MouseLayout,
) {
    let area = frame.area();
    let Some(popup) = picker_area(area) else {
        return;
    };

    let inner_width = popup.width.saturating_sub(4) as usize;
    let selected = app.selected_theme_index();
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Theme", theme::accent().add_modifier(Modifier::BOLD)),
        Span::styled("  live preview", theme::muted()),
    ]));
    lines.push(rule(inner_width));

    for (index, option) in theme::themes().iter().enumerate() {
        let selected_row = index == selected;
        let gutter = if selected_row { "▸" } else { " " };
        let name_width = 20.min(inner_width.saturating_sub(6));
        let description_width = inner_width.saturating_sub(name_width + 4);
        let name_style = if selected_row {
            theme::normal().add_modifier(Modifier::BOLD)
        } else {
            theme::normal()
        };

        let mut line = Line::from(vec![
            Span::styled(gutter, theme::accent().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(
                format!("{:<name_width$}", truncate(option.name, name_width)),
                name_style,
            ),
            Span::raw("  "),
            Span::styled(
                truncate(option.description, description_width),
                theme::muted(),
            ),
        ]);
        if selected_row {
            line = line.style(theme::selection());
        }
        lines.push(line);

        let row = popup.y.saturating_add(3).saturating_add(index as u16);
        if row < popup.bottom().saturating_sub(1) {
            mouse_layout.push(
                Rect::new(
                    popup.x.saturating_add(1),
                    row,
                    popup.width.saturating_sub(2),
                    1,
                ),
                MouseTarget::Theme(index),
            );
        }
    }

    lines.push(rule(inner_width));
    lines.push(preview_line(inner_width));
    lines.push(Line::from(vec![
        Span::styled("j/k", theme::muted_key()),
        Span::styled(" preview  ", theme::muted()),
        Span::styled("enter", theme::muted_key()),
        Span::styled(" save  ", theme::muted()),
        Span::styled("esc", theme::muted_key()),
        Span::styled(" cancel", theme::muted()),
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

pub(super) fn picker_area(area: Rect) -> Option<Rect> {
    if area.width < 40 || area.height < 15 {
        return None;
    }

    let width = ((area.width as f32 * 0.64) as u16).clamp(40, area.width);
    let height = (theme::theme_count().saturating_add(7) as u16).clamp(15, area.height);
    Some(Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    })
}

fn preview_line(width: usize) -> Line<'static> {
    let title = truncate("#42 Ship theme picker", width.saturating_sub(36).max(1));
    let padding = width.saturating_sub(title.chars().count() + 33).max(1);
    Line::from(vec![
        Span::styled("● ", theme::accent()),
        Span::styled("approved", theme::success().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(title, theme::normal().add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(padding)),
        Span::styled("2d", theme::warning()),
        Span::raw("   "),
        Span::styled("ci✓", theme::success()),
    ])
}

fn rule(width: usize) -> Line<'static> {
    Line::from(Span::styled("─".repeat(width.max(1)), theme::rule()))
}
