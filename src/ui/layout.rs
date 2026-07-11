use crate::app::DetailPane;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DetailLayout {
    pub description: Rect,
    pub discussion: Rect,
    pub footer: Rect,
}

impl DetailLayout {
    pub(super) fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(area);

        Self {
            description: chunks[0],
            discussion: chunks[1],
            footer: chunks[2],
        }
    }

    pub(super) fn pane_at_row(self, row: u16) -> Option<DetailPane> {
        if row >= self.description.y && row < self.description.bottom() {
            Some(DetailPane::Description)
        } else if row >= self.discussion.y && row < self.discussion.bottom() {
            Some(DetailPane::Discussion)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_layout_projects_regular_terminal() {
        let layout = DetailLayout::new(Rect::new(0, 0, 120, 40));
        assert_eq!(layout.description, Rect::new(0, 0, 120, 20));
        assert_eq!(layout.discussion, Rect::new(0, 20, 120, 18));
        assert_eq!(layout.footer, Rect::new(0, 38, 120, 2));
        assert_eq!(layout.pane_at_row(19), Some(DetailPane::Description));
        assert_eq!(layout.pane_at_row(20), Some(DetailPane::Discussion));
        assert_eq!(layout.pane_at_row(38), None);
    }

    #[test]
    fn detail_layout_preserves_ratatui_small_terminal_allocation() {
        let layout = DetailLayout::new(Rect::new(3, 5, 20, 2));
        assert_eq!(layout.description, Rect::new(3, 5, 20, 0));
        assert_eq!(layout.discussion, Rect::new(3, 5, 20, 1));
        assert_eq!(layout.footer, Rect::new(3, 6, 20, 1));
        assert_eq!(layout.pane_at_row(5), Some(DetailPane::Discussion));

        let layout = DetailLayout::new(Rect::new(0, 0, 20, 3));
        assert_eq!(layout.description, Rect::new(0, 0, 20, 0));
        assert_eq!(layout.discussion, Rect::new(0, 0, 20, 1));
        assert_eq!(layout.footer, Rect::new(0, 1, 20, 2));
        assert_eq!(layout.pane_at_row(0), Some(DetailPane::Discussion));
    }
}
