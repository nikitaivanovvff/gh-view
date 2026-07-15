use crate::app::{DashboardSection, DetailPane, ReviewScope};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};

pub(super) const DASHBOARD_MIN_SIZE: (u16, u16) = (40, 10);
pub(super) const DETAIL_MIN_SIZE: (u16, u16) = (40, 24);
pub(super) const SEARCH_MIN_SIZE: (u16, u16) = (40, 9);
pub(super) const THEME_PICKER_MIN_SIZE: (u16, u16) = (40, 15);
pub(super) const MOCK_DEBUG_MIN_SIZE: (u16, u16) = (40, 11);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MouseTarget {
    DashboardSection(DashboardSection),
    ReviewScope(ReviewScope),
    DashboardRow(usize),
    DetailPane(DetailPane),
    SearchMatch(usize),
    Theme(usize),
    DashboardRetry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HitRegion {
    area: Rect,
    target: MouseTarget,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct MouseLayout {
    regions: Vec<HitRegion>,
}

impl MouseLayout {
    pub(super) fn push(&mut self, area: Rect, target: MouseTarget) {
        if area.width > 0 && area.height > 0 {
            self.regions.push(HitRegion { area, target });
        }
    }

    pub(super) fn target_at(&self, position: Position) -> Option<MouseTarget> {
        self.regions
            .iter()
            .rev()
            .find(|region| region.area.contains(position))
            .map(|region| region.target)
    }
}

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
    }

    #[test]
    fn detail_layout_preserves_ratatui_small_terminal_allocation() {
        let layout = DetailLayout::new(Rect::new(3, 5, 20, 2));
        assert_eq!(layout.description, Rect::new(3, 5, 20, 0));
        assert_eq!(layout.discussion, Rect::new(3, 5, 20, 1));
        assert_eq!(layout.footer, Rect::new(3, 6, 20, 1));

        let layout = DetailLayout::new(Rect::new(0, 0, 20, 3));
        assert_eq!(layout.description, Rect::new(0, 0, 20, 0));
        assert_eq!(layout.discussion, Rect::new(0, 0, 20, 1));
        assert_eq!(layout.footer, Rect::new(0, 1, 20, 2));
    }

    #[test]
    fn mouse_layout_resolves_regions_and_edges() {
        let mut layout = MouseLayout::default();
        layout.push(Rect::new(3, 5, 4, 2), MouseTarget::DashboardRow(2));

        assert_eq!(
            layout.target_at(Position::new(3, 5)),
            Some(MouseTarget::DashboardRow(2))
        );
        assert_eq!(
            layout.target_at(Position::new(6, 6)),
            Some(MouseTarget::DashboardRow(2))
        );
        assert_eq!(layout.target_at(Position::new(7, 6)), None);
        assert_eq!(layout.target_at(Position::new(6, 7)), None);
        assert_eq!(layout.target_at(Position::new(0, 0)), None);
    }

    #[test]
    fn mouse_layout_ignores_empty_regions_and_prefers_last_overlap() {
        let mut layout = MouseLayout::default();
        layout.push(Rect::new(1, 1, 0, 2), MouseTarget::DashboardRow(0));
        layout.push(Rect::new(1, 1, 2, 0), MouseTarget::DashboardRow(0));
        layout.push(Rect::new(1, 1, 3, 3), MouseTarget::DashboardRow(1));
        layout.push(Rect::new(2, 2, 3, 3), MouseTarget::Theme(0));

        assert_eq!(
            layout.target_at(Position::new(1, 1)),
            Some(MouseTarget::DashboardRow(1))
        );
        assert_eq!(
            layout.target_at(Position::new(2, 2)),
            Some(MouseTarget::Theme(0))
        );
    }
}
