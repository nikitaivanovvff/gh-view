use super::palette::DEFAULT;
use ratatui::style::Style;

pub(super) fn normal() -> Style {
    Style::default().fg(DEFAULT.normal)
}

pub(super) fn muted() -> Style {
    Style::default().fg(DEFAULT.muted)
}

pub(super) fn accent() -> Style {
    Style::default().fg(DEFAULT.accent)
}

pub(super) fn rule() -> Style {
    Style::default().fg(DEFAULT.rule)
}

pub(super) fn focus_rule() -> Style {
    Style::default().fg(DEFAULT.focus_rule)
}

pub(super) fn selection() -> Style {
    Style::default().bg(DEFAULT.selection_bg)
}

pub(super) fn success() -> Style {
    Style::default().fg(DEFAULT.success)
}

pub(super) fn info() -> Style {
    Style::default().fg(DEFAULT.info)
}

pub(super) fn warning() -> Style {
    Style::default().fg(DEFAULT.warning)
}

pub(super) fn danger() -> Style {
    Style::default().fg(DEFAULT.danger)
}

pub(super) fn reviewer() -> Style {
    Style::default().fg(DEFAULT.reviewer)
}

pub(super) fn branch() -> Style {
    Style::default().fg(DEFAULT.branch)
}

pub(super) fn muted_key() -> Style {
    Style::default().fg(DEFAULT.muted_key)
}
