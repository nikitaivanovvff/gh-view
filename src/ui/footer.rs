use super::text::display_width;
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
    let mut used = 0;
    for item in items {
        let separator_width = usize::from(used > 0) * 3;
        let item_width = display_width(&item.key) + 1 + display_width(&item.label);
        if used + separator_width + item_width > width {
            continue;
        }
        if used > 0 {
            spans.push(Span::raw("   "));
            used += 3;
        }
        spans.push(Span::styled(item.key, theme::muted_key()));
        spans.push(Span::styled(format!(" {}", item.label), theme::muted()));
        used += item_width;
    }

    vec![rule_line(width), Line::from(spans)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_keeps_prioritized_controls_that_fit() {
        let footer = footer_lines(
            20,
            vec![
                FooterItem::new("q", "quit"),
                FooterItem::new("j/k", "move"),
                FooterItem::new("enter", "details"),
            ],
        )[1]
        .to_string();

        assert_eq!(footer, "q quit   j/k move");
        assert!(display_width(&footer) <= 20);
    }
}
