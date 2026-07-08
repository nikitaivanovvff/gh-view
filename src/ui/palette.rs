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
    pub branch: Color,
    pub muted_key: Color,
}

pub(super) const GH_VIEW: Palette = Palette {
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
    branch: Color::Rgb(107, 203, 191),
    muted_key: Color::Rgb(116, 111, 132),
};

pub(super) const CATPPUCCIN_MOCHA: Palette = Palette {
    normal: Color::Rgb(205, 214, 244),
    muted: Color::Rgb(127, 132, 156),
    accent: Color::Rgb(203, 166, 247),
    rule: Color::Rgb(49, 50, 68),
    focus_rule: Color::Rgb(180, 190, 254),
    selection_bg: Color::Rgb(49, 50, 68),
    success: Color::Rgb(166, 227, 161),
    info: Color::Rgb(137, 180, 250),
    warning: Color::Rgb(249, 226, 175),
    danger: Color::Rgb(243, 139, 168),
    reviewer: Color::Rgb(148, 226, 213),
    branch: Color::Rgb(137, 220, 235),
    muted_key: Color::Rgb(166, 173, 200),
};

pub(super) const TOKYO_NIGHT: Palette = Palette {
    normal: Color::Rgb(192, 202, 245),
    muted: Color::Rgb(86, 95, 137),
    accent: Color::Rgb(187, 154, 247),
    rule: Color::Rgb(41, 46, 66),
    focus_rule: Color::Rgb(122, 162, 247),
    selection_bg: Color::Rgb(36, 40, 59),
    success: Color::Rgb(158, 206, 106),
    info: Color::Rgb(125, 207, 255),
    warning: Color::Rgb(224, 175, 104),
    danger: Color::Rgb(247, 118, 142),
    reviewer: Color::Rgb(115, 218, 202),
    branch: Color::Rgb(125, 207, 255),
    muted_key: Color::Rgb(122, 162, 247),
};

pub(super) const ROSE_PINE: Palette = Palette {
    normal: Color::Rgb(224, 222, 244),
    muted: Color::Rgb(110, 106, 134),
    accent: Color::Rgb(196, 167, 231),
    rule: Color::Rgb(49, 43, 70),
    focus_rule: Color::Rgb(235, 188, 186),
    selection_bg: Color::Rgb(38, 35, 58),
    success: Color::Rgb(49, 116, 143),
    info: Color::Rgb(156, 207, 216),
    warning: Color::Rgb(246, 193, 119),
    danger: Color::Rgb(235, 111, 146),
    reviewer: Color::Rgb(156, 207, 216),
    branch: Color::Rgb(235, 188, 186),
    muted_key: Color::Rgb(144, 140, 170),
};

pub(super) const GRUVBOX_DARK: Palette = Palette {
    normal: Color::Rgb(235, 219, 178),
    muted: Color::Rgb(146, 131, 116),
    accent: Color::Rgb(211, 134, 155),
    rule: Color::Rgb(60, 56, 54),
    focus_rule: Color::Rgb(250, 189, 47),
    selection_bg: Color::Rgb(50, 48, 47),
    success: Color::Rgb(184, 187, 38),
    info: Color::Rgb(131, 165, 152),
    warning: Color::Rgb(250, 189, 47),
    danger: Color::Rgb(251, 73, 52),
    reviewer: Color::Rgb(142, 192, 124),
    branch: Color::Rgb(104, 157, 106),
    muted_key: Color::Rgb(168, 153, 132),
};
