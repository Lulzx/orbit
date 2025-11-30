//! Footer widget with keybindings

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::Widget,
};

use crate::core::state::{AppMode, AppState};
use crate::ui::theme::Theme;

pub struct Footer<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> Footer<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl<'a> Widget for Footer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.theme.styles.footer);

        let bindings = match &self.state.mode {
            AppMode::Dashboard => vec![
                ("Space", "Actions"),
                ("e", "Env"),
                ("d", "Docker"),
                ("p", "Ports"),
                ("f", "Focus"),
                ("r", "Refresh"),
                ("?", "Help"),
                ("q", "Quit"),
            ],
            AppMode::CommandPalette => vec![
                ("↑/↓", "Navigate"),
                ("Enter", "Execute"),
                ("Esc", "Close"),
            ],
            AppMode::Help => vec![
                ("Esc", "Close"),
                ("q", "Close"),
            ],
            AppMode::FocusMode { .. } => vec![
                ("Esc", "Exit Focus"),
            ],
            AppMode::Confirm { .. } => vec![
                ("y", "Confirm"),
                ("n", "Cancel"),
                ("Esc", "Cancel"),
            ],
            _ => vec![],
        };

        let mut spans = Vec::new();
        for (i, (key, action)) in bindings.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", self.theme.styles.keybind));
            }
            spans.push(Span::styled(
                format!("[{}]", key),
                self.theme.styles.keybind_key,
            ));
            spans.push(Span::styled(
                format!(" {}", action),
                self.theme.styles.keybind,
            ));
        }

        let line = Line::from(spans);
        buf.set_line(area.x + 1, area.y, &line, area.width.saturating_sub(2));
    }
}
