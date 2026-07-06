use super::text::rule_line;
use super::theme;
use ratatui::text::{Line, Span};

pub(super) struct FooterItem {
    key: String,
    label: String,
}

impl FooterItem {
    pub(super) fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
        }
    }
}

pub(super) fn footer_lines(width: usize, items: Vec<FooterItem>) -> Vec<Line<'static>> {
    let mut spans = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(item.key, theme::muted_key()));
        spans.push(Span::styled(format!(" {}", item.label), theme::muted()));
    }

    vec![rule_line(width), Line::from(spans)]
}
