use super::palette::{
    CATPPUCCIN_LATTE, CATPPUCCIN_MOCHA, GH_VIEW, GITHUB_LIGHT, GRUVBOX_DARK, Palette, ROSE_PINE,
    SOLARIZED_LIGHT, TOKYO_NIGHT,
};
use ratatui::style::Style;
use std::sync::atomic::{AtomicUsize, Ordering};

static ACTIVE_THEME: AtomicUsize = AtomicUsize::new(0);

pub(super) struct ThemeOption {
    pub(super) key: &'static str,
    pub(super) name: &'static str,
    pub(super) description: &'static str,
    palette: &'static Palette,
}

const THEMES: &[ThemeOption] = &[
    ThemeOption {
        key: "default",
        name: "default",
        description: "default purple terminal palette",
        palette: &GH_VIEW,
    },
    ThemeOption {
        key: "catppuccin-mocha",
        name: "Catppuccin Mocha",
        description: "cozy pastel dark theme",
        palette: &CATPPUCCIN_MOCHA,
    },
    ThemeOption {
        key: "tokyo-night",
        name: "Tokyo Night",
        description: "clean neon city contrast",
        palette: &TOKYO_NIGHT,
    },
    ThemeOption {
        key: "rose-pine",
        name: "Rosé Pine",
        description: "soft rose-tinted dark palette",
        palette: &ROSE_PINE,
    },
    ThemeOption {
        key: "gruvbox-dark",
        name: "Gruvbox Dark",
        description: "warm retro developer colors",
        palette: &GRUVBOX_DARK,
    },
    ThemeOption {
        key: "catppuccin-latte",
        name: "Catppuccin Latte",
        description: "warm pastel light theme",
        palette: &CATPPUCCIN_LATTE,
    },
    ThemeOption {
        key: "solarized-light",
        name: "Solarized Light",
        description: "low-contrast precision light theme",
        palette: &SOLARIZED_LIGHT,
    },
    ThemeOption {
        key: "github-light",
        name: "GitHub Light",
        description: "crisp neutral light theme",
        palette: &GITHUB_LIGHT,
    },
];

pub(super) fn themes() -> &'static [ThemeOption] {
    THEMES
}

pub(super) fn theme_count() -> usize {
    THEMES.len()
}

pub(super) fn theme_key(index: usize) -> Option<&'static str> {
    THEMES.get(index).map(|theme| theme.key)
}

pub(super) fn theme_index(value: &str) -> usize {
    THEMES
        .iter()
        .position(|theme| {
            theme.key.eq_ignore_ascii_case(value) || theme.name.eq_ignore_ascii_case(value)
        })
        .unwrap_or(0)
}

pub(super) fn set_active_theme(index: usize) {
    ACTIVE_THEME.store(index.min(THEMES.len().saturating_sub(1)), Ordering::Relaxed);
}

fn active() -> &'static Palette {
    THEMES
        .get(ACTIVE_THEME.load(Ordering::Relaxed))
        .map(|theme| theme.palette)
        .unwrap_or(&GH_VIEW)
}

pub(super) fn background() -> Style {
    Style::default().bg(active().background)
}

pub(super) fn normal() -> Style {
    Style::default().fg(active().normal)
}

pub(super) fn muted() -> Style {
    Style::default().fg(active().muted)
}

pub(super) fn accent() -> Style {
    Style::default().fg(active().accent)
}

pub(super) fn rule() -> Style {
    Style::default().fg(active().rule)
}

pub(super) fn focus_rule() -> Style {
    Style::default().fg(active().focus_rule)
}

pub(super) fn selection() -> Style {
    Style::default().bg(active().selection_bg)
}

pub(super) fn success() -> Style {
    Style::default().fg(active().success)
}

pub(super) fn info() -> Style {
    Style::default().fg(active().info)
}

pub(super) fn warning() -> Style {
    Style::default().fg(active().warning)
}

pub(super) fn danger() -> Style {
    Style::default().fg(active().danger)
}

pub(super) fn reviewer() -> Style {
    Style::default().fg(active().reviewer)
}

pub(super) fn branch() -> Style {
    Style::default().fg(active().branch)
}

pub(super) fn muted_key() -> Style {
    Style::default().fg(active().muted_key)
}
