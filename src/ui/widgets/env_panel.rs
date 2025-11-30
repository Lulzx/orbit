//! Environment variables panel

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::core::state::{AppState, EnvSource, FocusedPanel};
use crate::ui::theme::Theme;

pub struct EnvPanel<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> EnvPanel<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }

    fn redact_value(value: &str) -> String {
        if value.len() <= 4 {
            "●".repeat(value.len())
        } else {
            format!(
                "{}{}{}",
                &value[..2],
                "●".repeat((value.len() - 4).min(12)),
                &value[value.len() - 2..]
            )
        }
    }
}

impl<'a> Widget for EnvPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let focused = self.state.focus_panel == FocusedPanel::Env;
        let border_style = if focused {
            self.theme.styles.panel_border_focused
        } else {
            self.theme.styles.panel_border
        };

        let block = Block::default()
            .title(Span::styled(" ENVIRONMENT ", self.theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(self.theme.colors.bg_primary));

        let inner = block.inner(area);
        block.render(area, buf);

        let env = &self.state.panels.env;
        let selected = env.selected_index;
        let show_values = env.show_values;

        if env.variables.is_empty() {
            let span = Span::styled("No environment variables", Style::default().fg(self.theme.colors.fg_muted));
            buf.set_span(inner.x + 1, inner.y, &span, inner.width.saturating_sub(2));
            return;
        }

        for (i, var) in env.variables.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let is_selected = i == selected;
            let base_style = if is_selected {
                self.theme.styles.list_item_selected
            } else {
                self.theme.styles.list_item
            };

            let indicator = if is_selected { "▸" } else { " " };
            
            let (status_icon, status_style) = match var.source {
                EnvSource::Shell => ("✓", self.theme.styles.status_running),
                EnvSource::DotEnv => ("●", Style::default().fg(self.theme.colors.info)),
                EnvSource::Keychain => ("🔐", self.theme.styles.status_running),
                EnvSource::Missing => ("⚠", self.theme.styles.status_warning),
            };

            let source_label = match var.source {
                EnvSource::Shell => "shell",
                EnvSource::DotEnv => ".env",
                EnvSource::Keychain => "keychain",
                EnvSource::Missing => "MISSING",
            };

            let name = truncate(&var.name, 16);
            
            let value_display = if var.source == EnvSource::Missing {
                "".to_string()
            } else if show_values && !var.is_secret {
                var.value.as_deref().map(|v| truncate(v, 20)).unwrap_or_default()
            } else {
                var.value.as_deref()
                    .map(Self::redact_value)
                    .unwrap_or_default()
            };

            let line = Line::from(vec![
                Span::styled(indicator, base_style),
                Span::styled(format!(" {:<16} ", name), base_style),
                Span::styled(format!("{:<20} ", value_display), self.theme.styles.secret_redacted),
                Span::styled(format!("{} ", status_icon), status_style),
                Span::styled(source_label, Style::default().fg(self.theme.colors.fg_muted)),
            ]);

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 1 {
        format!("{}…", &s[..max_len - 1])
    } else {
        String::new()
    }
}
