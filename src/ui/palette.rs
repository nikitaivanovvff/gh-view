use ratatui::style::Color;

pub(super) struct Palette {
    pub background: Color,
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
    background: Color::Rgb(18, 17, 23),
    normal: Color::Rgb(214, 211, 221),
    muted: Color::Rgb(134, 132, 143),
    accent: Color::Rgb(152, 102, 255),
    rule: Color::Rgb(48, 45, 57),
    focus_rule: Color::Rgb(116, 111, 132),
    selection_bg: Color::Rgb(31, 29, 38),
    success: Color::Rgb(35, 209, 139),
    info: Color::Rgb(86, 156, 214),
    warning: Color::Rgb(220, 170, 88),
    danger: Color::Rgb(232, 93, 117),
    reviewer: Color::Rgb(64, 196, 150),
    branch: Color::Rgb(107, 203, 191),
    muted_key: Color::Rgb(135, 131, 149),
};

pub(super) const CATPPUCCIN_MOCHA: Palette = Palette {
    background: Color::Rgb(30, 30, 46),
    normal: Color::Rgb(205, 214, 244),
    muted: Color::Rgb(150, 154, 173),
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
    background: Color::Rgb(26, 27, 38),
    normal: Color::Rgb(192, 202, 245),
    muted: Color::Rgb(136, 142, 172),
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
    background: Color::Rgb(25, 23, 36),
    normal: Color::Rgb(224, 222, 244),
    muted: Color::Rgb(141, 138, 160),
    accent: Color::Rgb(196, 167, 231),
    rule: Color::Rgb(49, 43, 70),
    focus_rule: Color::Rgb(235, 188, 186),
    selection_bg: Color::Rgb(38, 35, 58),
    success: Color::Rgb(96, 147, 168),
    info: Color::Rgb(156, 207, 216),
    warning: Color::Rgb(246, 193, 119),
    danger: Color::Rgb(235, 111, 146),
    reviewer: Color::Rgb(156, 207, 216),
    branch: Color::Rgb(235, 188, 186),
    muted_key: Color::Rgb(144, 140, 170),
};

pub(super) const GRUVBOX_DARK: Palette = Palette {
    background: Color::Rgb(40, 40, 40),
    normal: Color::Rgb(235, 219, 178),
    muted: Color::Rgb(162, 150, 137),
    accent: Color::Rgb(211, 134, 155),
    rule: Color::Rgb(60, 56, 54),
    focus_rule: Color::Rgb(250, 189, 47),
    selection_bg: Color::Rgb(50, 48, 47),
    success: Color::Rgb(184, 187, 38),
    info: Color::Rgb(131, 165, 152),
    warning: Color::Rgb(250, 189, 47),
    danger: Color::Rgb(252, 103, 85),
    reviewer: Color::Rgb(142, 192, 124),
    branch: Color::Rgb(114, 163, 115),
    muted_key: Color::Rgb(168, 153, 132),
};

pub(super) const CATPPUCCIN_LATTE: Palette = Palette {
    background: Color::Rgb(239, 241, 245),
    normal: Color::Rgb(76, 79, 105),
    muted: Color::Rgb(96, 99, 114),
    accent: Color::Rgb(128, 54, 224),
    rule: Color::Rgb(204, 208, 218),
    focus_rule: Color::Rgb(101, 119, 224),
    selection_bg: Color::Rgb(220, 224, 232),
    success: Color::Rgb(45, 113, 30),
    info: Color::Rgb(27, 90, 217),
    warning: Color::Rgb(139, 88, 18),
    danger: Color::Rgb(198, 14, 54),
    reviewer: Color::Rgb(17, 110, 116),
    branch: Color::Rgb(3, 106, 148),
    muted_key: Color::Rgb(96, 98, 118),
};

pub(super) const SOLARIZED_LIGHT: Palette = Palette {
    background: Color::Rgb(253, 246, 227),
    normal: Color::Rgb(87, 108, 115),
    muted: Color::Rgb(95, 107, 108),
    accent: Color::Rgb(94, 98, 170),
    rule: Color::Rgb(238, 232, 213),
    focus_rule: Color::Rgb(38, 139, 210),
    selection_bg: Color::Rgb(238, 232, 213),
    success: Color::Rgb(97, 111, 0),
    info: Color::Rgb(30, 109, 165),
    warning: Color::Rgb(132, 100, 0),
    danger: Color::Rgb(197, 45, 42),
    reviewer: Color::Rgb(30, 116, 110),
    branch: Color::Rgb(30, 109, 165),
    muted_key: Color::Rgb(88, 108, 115),
};

pub(super) const GITHUB_LIGHT: Palette = Palette {
    background: Color::Rgb(255, 255, 255),
    normal: Color::Rgb(31, 35, 40),
    muted: Color::Rgb(89, 99, 110),
    accent: Color::Rgb(129, 79, 221),
    rule: Color::Rgb(209, 217, 224),
    focus_rule: Color::Rgb(9, 105, 218),
    selection_bg: Color::Rgb(221, 244, 255),
    success: Color::Rgb(26, 126, 55),
    info: Color::Rgb(9, 105, 218),
    warning: Color::Rgb(149, 100, 0),
    danger: Color::Rgb(209, 36, 47),
    reviewer: Color::Rgb(18, 122, 122),
    branch: Color::Rgb(9, 105, 218),
    muted_key: Color::Rgb(89, 99, 110),
};

#[cfg(test)]
mod tests {
    use super::*;

    const PALETTES: [(&str, &Palette); 8] = [
        ("default", &GH_VIEW),
        ("catppuccin-mocha", &CATPPUCCIN_MOCHA),
        ("tokyo-night", &TOKYO_NIGHT),
        ("rose-pine", &ROSE_PINE),
        ("gruvbox-dark", &GRUVBOX_DARK),
        ("catppuccin-latte", &CATPPUCCIN_LATTE),
        ("solarized-light", &SOLARIZED_LIGHT),
        ("github-light", &GITHUB_LIGHT),
    ];

    #[test]
    fn informational_roles_meet_text_contrast() {
        for (palette_name, palette) in PALETTES {
            for (role, color) in text_roles(palette) {
                assert_contrast(palette_name, role, color, palette.background, 4.5);
            }
        }
    }

    #[test]
    fn focus_rules_meet_boundary_contrast() {
        for (palette_name, palette) in PALETTES {
            assert_contrast(
                palette_name,
                "focus_rule",
                palette.focus_rule,
                palette.background,
                3.0,
            );
        }
    }

    #[test]
    fn selection_foregrounds_remain_readable() {
        for (palette_name, palette) in PALETTES {
            for (role, color) in [
                ("normal", palette.normal),
                ("muted", palette.muted),
                ("accent", palette.accent),
                ("success", palette.success),
                ("danger", palette.danger),
            ] {
                assert_contrast(palette_name, role, color, palette.selection_bg, 4.5);
            }
            assert_contrast(
                palette_name,
                "focus_rule",
                palette.focus_rule,
                palette.selection_bg,
                3.0,
            );
        }
    }

    fn text_roles(palette: &Palette) -> [(&'static str, Color); 10] {
        [
            ("normal", palette.normal),
            ("muted", palette.muted),
            ("muted_key", palette.muted_key),
            ("accent", palette.accent),
            ("success", palette.success),
            ("info", palette.info),
            ("warning", palette.warning),
            ("danger", palette.danger),
            ("reviewer", palette.reviewer),
            ("branch", palette.branch),
        ]
    }

    fn assert_contrast(name: &str, role: &str, foreground: Color, background: Color, minimum: f64) {
        let ratio = contrast_ratio(foreground, background);
        assert!(
            ratio + f64::EPSILON >= minimum,
            "{name} {role} contrast {ratio:.2} is below {minimum:.1}"
        );
    }

    fn contrast_ratio(left: Color, right: Color) -> f64 {
        let left = luminance(left);
        let right = luminance(right);
        (left.max(right) + 0.05) / (left.min(right) + 0.05)
    }

    fn luminance(color: Color) -> f64 {
        let Color::Rgb(red, green, blue) = color else {
            panic!("palette colors must use explicit RGB values");
        };
        0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
    }

    fn linear(channel: u8) -> f64 {
        let channel = f64::from(channel) / 255.0;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }
}
