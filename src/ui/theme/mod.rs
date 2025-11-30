//! Theme system with beautiful color palettes

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

/// Complete theme definition
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
    pub styles: ThemeStyles,
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Base colors
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_tertiary: Color,
    pub bg_highlight: Color,

    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_muted: Color,

    // Accent colors
    pub accent_primary: Color,
    pub accent_secondary: Color,

    // Semantic colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Special
    pub border: Color,
    pub border_focused: Color,
    pub selection: Color,
    pub cursor: Color,
}

#[derive(Debug, Clone)]
pub struct ThemeStyles {
    pub header: Style,
    pub footer: Style,
    pub panel_title: Style,
    pub panel_border: Style,
    pub panel_border_focused: Style,
    pub list_item: Style,
    pub list_item_selected: Style,
    pub status_running: Style,
    pub status_stopped: Style,
    pub status_warning: Style,
    pub sparkline: Style,
    pub keybind: Style,
    pub keybind_key: Style,
    pub secret_redacted: Style,
    pub notification_info: Style,
    pub notification_success: Style,
    pub notification_error: Style,
}

impl Theme {
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "catppuccin" | "catppuccin-mocha" => Self::catppuccin_mocha(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "gruvbox" => Self::gruvbox(),
            _ => Self::tokyo_night(), // Default
        }
    }

    /// Tokyo Night theme (default)
    pub fn tokyo_night() -> Self {
        let colors = ThemeColors {
            bg_primary: Color::Rgb(26, 27, 38),
            bg_secondary: Color::Rgb(36, 40, 59),
            bg_tertiary: Color::Rgb(41, 46, 66),
            bg_highlight: Color::Rgb(47, 53, 73),

            fg_primary: Color::Rgb(192, 202, 245),
            fg_secondary: Color::Rgb(169, 177, 214),
            fg_muted: Color::Rgb(86, 95, 137),

            accent_primary: Color::Rgb(122, 162, 247),
            accent_secondary: Color::Rgb(187, 154, 247),

            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            error: Color::Rgb(247, 118, 142),
            info: Color::Rgb(125, 207, 255),

            border: Color::Rgb(41, 46, 66),
            border_focused: Color::Rgb(122, 162, 247),
            selection: Color::Rgb(52, 59, 88),
            cursor: Color::Rgb(192, 202, 245),
        };

        Self::from_colors("Tokyo Night", colors)
    }

    /// Catppuccin Mocha theme
    pub fn catppuccin_mocha() -> Self {
        let colors = ThemeColors {
            bg_primary: Color::Rgb(30, 30, 46),
            bg_secondary: Color::Rgb(49, 50, 68),
            bg_tertiary: Color::Rgb(69, 71, 90),
            bg_highlight: Color::Rgb(88, 91, 112),

            fg_primary: Color::Rgb(205, 214, 244),
            fg_secondary: Color::Rgb(186, 194, 222),
            fg_muted: Color::Rgb(147, 153, 178),

            accent_primary: Color::Rgb(137, 180, 250),
            accent_secondary: Color::Rgb(203, 166, 247),

            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            info: Color::Rgb(148, 226, 213),

            border: Color::Rgb(69, 71, 90),
            border_focused: Color::Rgb(137, 180, 250),
            selection: Color::Rgb(88, 91, 112),
            cursor: Color::Rgb(205, 214, 244),
        };

        Self::from_colors("Catppuccin Mocha", colors)
    }

    /// Dracula theme
    pub fn dracula() -> Self {
        let colors = ThemeColors {
            bg_primary: Color::Rgb(40, 42, 54),
            bg_secondary: Color::Rgb(68, 71, 90),
            bg_tertiary: Color::Rgb(98, 114, 164),
            bg_highlight: Color::Rgb(68, 71, 90),

            fg_primary: Color::Rgb(248, 248, 242),
            fg_secondary: Color::Rgb(189, 147, 249),
            fg_muted: Color::Rgb(98, 114, 164),

            accent_primary: Color::Rgb(139, 233, 253),
            accent_secondary: Color::Rgb(255, 121, 198),

            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(255, 184, 108),
            error: Color::Rgb(255, 85, 85),
            info: Color::Rgb(139, 233, 253),

            border: Color::Rgb(68, 71, 90),
            border_focused: Color::Rgb(189, 147, 249),
            selection: Color::Rgb(68, 71, 90),
            cursor: Color::Rgb(248, 248, 242),
        };

        Self::from_colors("Dracula", colors)
    }

    /// Nord theme
    pub fn nord() -> Self {
        let colors = ThemeColors {
            bg_primary: Color::Rgb(46, 52, 64),
            bg_secondary: Color::Rgb(59, 66, 82),
            bg_tertiary: Color::Rgb(67, 76, 94),
            bg_highlight: Color::Rgb(76, 86, 106),

            fg_primary: Color::Rgb(236, 239, 244),
            fg_secondary: Color::Rgb(229, 233, 240),
            fg_muted: Color::Rgb(216, 222, 233),

            accent_primary: Color::Rgb(136, 192, 208),
            accent_secondary: Color::Rgb(129, 161, 193),

            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            error: Color::Rgb(191, 97, 106),
            info: Color::Rgb(136, 192, 208),

            border: Color::Rgb(67, 76, 94),
            border_focused: Color::Rgb(136, 192, 208),
            selection: Color::Rgb(76, 86, 106),
            cursor: Color::Rgb(236, 239, 244),
        };

        Self::from_colors("Nord", colors)
    }

    /// Gruvbox theme
    pub fn gruvbox() -> Self {
        let colors = ThemeColors {
            bg_primary: Color::Rgb(40, 40, 40),
            bg_secondary: Color::Rgb(60, 56, 54),
            bg_tertiary: Color::Rgb(80, 73, 69),
            bg_highlight: Color::Rgb(102, 92, 84),

            fg_primary: Color::Rgb(235, 219, 178),
            fg_secondary: Color::Rgb(213, 196, 161),
            fg_muted: Color::Rgb(168, 153, 132),

            accent_primary: Color::Rgb(131, 165, 152),
            accent_secondary: Color::Rgb(211, 134, 155),

            success: Color::Rgb(184, 187, 38),
            warning: Color::Rgb(250, 189, 47),
            error: Color::Rgb(251, 73, 52),
            info: Color::Rgb(131, 165, 152),

            border: Color::Rgb(80, 73, 69),
            border_focused: Color::Rgb(131, 165, 152),
            selection: Color::Rgb(102, 92, 84),
            cursor: Color::Rgb(235, 219, 178),
        };

        Self::from_colors("Gruvbox", colors)
    }

    fn from_colors(name: &str, colors: ThemeColors) -> Self {
        let styles = ThemeStyles {
            header: Style::default()
                .bg(colors.bg_secondary)
                .fg(colors.fg_primary),
            footer: Style::default().bg(colors.bg_secondary).fg(colors.fg_muted),
            panel_title: Style::default()
                .fg(colors.accent_primary)
                .add_modifier(Modifier::BOLD),
            panel_border: Style::default().fg(colors.border),
            panel_border_focused: Style::default().fg(colors.border_focused),
            list_item: Style::default().fg(colors.fg_primary),
            list_item_selected: Style::default()
                .fg(colors.fg_primary)
                .bg(colors.selection)
                .add_modifier(Modifier::BOLD),
            status_running: Style::default().fg(colors.success),
            status_stopped: Style::default().fg(colors.error),
            status_warning: Style::default().fg(colors.warning),
            sparkline: Style::default().fg(colors.accent_primary),
            keybind: Style::default().fg(colors.fg_muted),
            keybind_key: Style::default()
                .fg(colors.accent_secondary)
                .add_modifier(Modifier::BOLD),
            secret_redacted: Style::default()
                .fg(colors.fg_muted)
                .add_modifier(Modifier::DIM),
            notification_info: Style::default().fg(colors.info),
            notification_success: Style::default().fg(colors.success),
            notification_error: Style::default().fg(colors.error),
        };

        Self {
            name: name.to_string(),
            colors,
            styles,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::tokyo_night()
    }
}
