//! Actions panel widget (quick actions list)

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::core::state::{AppState, FocusedPanel};
use crate::actions::ActionCategory;
use crate::ui::theme::Theme;

pub struct ActionsPanel<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> ActionsPanel<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }

    fn category_icon(category: &ActionCategory) -> &'static str {
        match category {
            ActionCategory::Dev => "▶",
            ActionCategory::Build => "⚙",
            ActionCategory::Test => "✓",
            ActionCategory::Lint => "◉",
            ActionCategory::Deploy => "↑",
            ActionCategory::Database => "⬡",
            ActionCategory::Docker => "🐳",
            ActionCategory::System => "⚡",
            ActionCategory::Custom => "★",
        }
    }
}

impl<'a> Widget for ActionsPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let focused = self.state.focus_panel == FocusedPanel::Actions;
        let border_style = if focused {
            self.theme.styles.panel_border_focused
        } else {
            self.theme.styles.panel_border
        };

        let block = Block::default()
            .title(Span::styled(" QUICK ACTIONS ", self.theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(self.theme.colors.bg_primary));

        let inner = block.inner(area);
        block.render(area, buf);

        let actions = &self.state.panels.actions;
        let selected = actions.selected_index;

        if actions.filtered_indices.is_empty() {
            let msg = if actions.actions.is_empty() {
                "No actions detected"
            } else {
                "No matching actions"
            };
            let span = Span::styled(msg, Style::default().fg(self.theme.colors.fg_muted));
            buf.set_span(inner.x + 1, inner.y, &span, inner.width.saturating_sub(2));
            return;
        }

        for (display_idx, &action_idx) in actions.filtered_indices.iter().enumerate() {
            if display_idx >= inner.height as usize {
                break;
            }

            let action = &actions.actions[action_idx];
            let is_selected = display_idx == selected;

            let base_style = if is_selected {
                self.theme.styles.list_item_selected
            } else {
                self.theme.styles.list_item
            };

            let indicator = if is_selected { "▸" } else { " " };
            let icon = Self::category_icon(&action.category);
            let name = truncate(&action.name, 20);
            let desc = action.description.as_deref()
                .or(Some(&action.command))
                .map(|s| truncate(s, inner.width.saturating_sub(28) as usize))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(indicator, base_style),
                Span::styled(format!(" {} ", icon), Style::default().fg(self.theme.colors.accent_secondary)),
                Span::styled(format!("{:<20} ", name), base_style),
                Span::styled(desc, Style::default().fg(self.theme.colors.fg_muted)),
            ]);

            buf.set_line(inner.x, inner.y + display_idx as u16, &line, inner.width);
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
