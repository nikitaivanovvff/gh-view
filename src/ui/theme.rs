use ratatui::style::{Color, Style};

pub(super) fn normal() -> Style {
    Style::default().fg(Color::Rgb(214, 211, 221))
}

pub(super) fn muted() -> Style {
    Style::default().fg(Color::Rgb(91, 88, 103))
}

pub(super) fn accent() -> Style {
    Style::default().fg(Color::Rgb(137, 81, 255))
}

pub(super) fn rule() -> Style {
    Style::default().fg(Color::Rgb(48, 45, 57))
}

pub(super) fn focus_rule() -> Style {
    Style::default().fg(Color::Rgb(116, 111, 132))
}

pub(super) fn selection() -> Style {
    Style::default().bg(Color::Rgb(31, 29, 38))
}

pub(super) fn success() -> Style {
    Style::default().fg(Color::Rgb(35, 209, 139))
}

pub(super) fn info() -> Style {
    Style::default().fg(Color::Rgb(86, 156, 214))
}

pub(super) fn warning() -> Style {
    Style::default().fg(Color::Rgb(220, 170, 88))
}

pub(super) fn danger() -> Style {
    Style::default().fg(Color::Rgb(232, 93, 117))
}

pub(super) fn reviewer() -> Style {
    Style::default().fg(Color::Rgb(64, 196, 150))
}

pub(super) fn muted_key() -> Style {
    Style::default().fg(Color::Rgb(116, 111, 132))
}
