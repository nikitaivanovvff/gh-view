use ratatui::style::Color;

pub(super) struct Palette {
    pub normal: Color,
    pub muted: Color,
    pub accent: Color,
    pub rule: Color,
    pub focus_rule: Color,
    pub selection_bg: Color,
    pub success: Color,
    pub info: Color,
    pub warning: Color,
    pub danger: Color,
    pub reviewer: Color,
    pub muted_key: Color,
}

pub(super) const DEFAULT: Palette = Palette {
    normal: Color::Rgb(214, 211, 221),
    muted: Color::Rgb(91, 88, 103),
    accent: Color::Rgb(137, 81, 255),
    rule: Color::Rgb(48, 45, 57),
    focus_rule: Color::Rgb(116, 111, 132),
    selection_bg: Color::Rgb(31, 29, 38),
    success: Color::Rgb(35, 209, 139),
    info: Color::Rgb(86, 156, 214),
    warning: Color::Rgb(220, 170, 88),
    danger: Color::Rgb(232, 93, 117),
    reviewer: Color::Rgb(64, 196, 150),
    muted_key: Color::Rgb(116, 111, 132),
};
