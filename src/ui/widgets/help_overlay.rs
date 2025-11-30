//! Help overlay widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::ui::theme::Theme;

pub struct HelpOverlay<'a> {
    theme: &'a Theme,
}

impl<'a> HelpOverlay<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }
}

impl<'a> Widget for HelpOverlay<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let block = Block::default()
            .title(Span::styled(" 🛰️ Orbit Help ", self.theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(self.theme.styles.panel_border_focused)
            .style(Style::default().bg(self.theme.colors.bg_secondary));

        let inner = block.inner(area);
        block.render(area, buf);

        let keybindings = [
            ("General", vec![
                ("q", "Quit Orbit"),
                ("?", "Toggle help"),
                ("r", "Refresh project detection"),
                ("Tab", "Cycle panel focus"),
            ]),
            ("Navigation", vec![
                ("↑/k", "Move up"),
                ("↓/j", "Move down"),
                ("Enter", "Execute selected action"),
                ("Space", "Open command palette"),
            ]),
            ("Panels", vec![
                ("d", "Toggle Docker panel"),
                ("p", "Toggle Ports panel"),
                ("e", "Toggle Environment panel"),
            ]),
            ("Modes", vec![
                ("f", "Enter focus mode"),
                ("Esc", "Exit current mode / Close overlay"),
            ]),
        ];

        let mut y = inner.y;

        for (section, bindings) in &keybindings {
            if y >= inner.y + inner.height {
                break;
            }

            // Section header
            let header = Line::from(vec![Span::styled(
                format!("─── {} ", section),
                Style::default()
                    .fg(self.theme.colors.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )]);
            buf.set_line(inner.x + 1, y, &header, inner.width.saturating_sub(2));
            y += 1;

            // Bindings
            for (key, desc) in bindings {
                if y >= inner.y + inner.height {
                    break;
                }

                let line = Line::from(vec![
                    Span::styled(
                        format!("  {:>8}  ", key),
                        self.theme.styles.keybind_key,
                    ),
                    Span::styled(*desc, self.theme.styles.keybind),
                ]);
                buf.set_line(inner.x + 1, y, &line, inner.width.saturating_sub(2));
                y += 1;
            }

            y += 1; // Space between sections
        }

        // Footer
        let footer_y = area.y + area.height - 1;
        let footer = Span::styled(
            " Press Esc or ? to close ",
            Style::default().fg(self.theme.colors.fg_muted),
        );
        buf.set_span(
            area.x + (area.width - 25) / 2,
            footer_y,
            &footer,
            25,
        );
    }
}
