//! Command palette overlay

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::core::state::AppState;
use crate::actions::ActionCategory;
use crate::ui::theme::Theme;

pub struct ActionPalette<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> ActionPalette<'a> {
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

impl<'a> Widget for ActionPalette<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear background
        Clear.render(area, buf);

        // Main container
        let block = Block::default()
            .title(Span::styled(" ⚡ Quick Actions ", self.theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(self.theme.styles.panel_border_focused)
            .style(Style::default().bg(self.theme.colors.bg_secondary));

        let inner = block.inner(area);
        block.render(area, buf);

        // Split: search input | results
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Search input
                Constraint::Min(3),    // Results list
            ])
            .split(inner);

        // Search input area
        let actions = &self.state.panels.actions;
        let filter = &actions.filter;

        let search_text = if filter.is_empty() {
            Span::styled(
                "Type to filter...",
                Style::default().fg(self.theme.colors.fg_muted),
            )
        } else {
            Span::styled(filter, Style::default().fg(self.theme.colors.fg_primary))
        };

        let search_line = Line::from(vec![
            Span::styled(
                " 🔍 ",
                Style::default().fg(self.theme.colors.accent_primary),
            ),
            search_text,
            Span::styled("█", Style::default().fg(self.theme.colors.cursor)),
        ]);

        buf.set_line(chunks[0].x, chunks[0].y, &search_line, chunks[0].width);

        // Draw separator
        let sep_y = chunks[0].y + 1;
        for x in chunks[0].x..chunks[0].x + chunks[0].width {
            if let Some(cell) = buf.cell_mut((x, sep_y)) {
                cell.set_char('─')
                    .set_style(self.theme.styles.panel_border);
            }
        }

        // Results list
        let results_area = chunks[1];
        let selected = actions.selected_index;

        if actions.filtered_indices.is_empty() {
            let msg = if actions.actions.is_empty() {
                "No actions available"
            } else {
                "No matching actions"
            };
            let span = Span::styled(msg, Style::default().fg(self.theme.colors.fg_muted));
            buf.set_span(
                results_area.x + 2,
                results_area.y,
                &span,
                results_area.width.saturating_sub(4),
            );
            return;
        }

        for (display_idx, &action_idx) in actions.filtered_indices.iter().enumerate() {
            if display_idx >= results_area.height as usize {
                break;
            }

            let action = &actions.actions[action_idx];
            let is_selected = display_idx == selected;

            let style = if is_selected {
                self.theme.styles.list_item_selected
            } else {
                self.theme.styles.list_item
            };

            let indicator = if is_selected { "▸" } else { " " };
            let icon = Self::category_icon(&action.category);

            // Highlight matching parts in name
            let name = &action.name;
            let desc = action
                .description
                .as_deref()
                .unwrap_or(&action.command);

            let max_name_len = 24;
            let max_desc_len = results_area.width.saturating_sub(max_name_len as u16 + 8) as usize;

            let name_display = truncate(name, max_name_len);
            let desc_display = truncate(desc, max_desc_len);

            let line = Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(
                    format!(" {} ", icon),
                    Style::default().fg(self.theme.colors.accent_secondary),
                ),
                Span::styled(
                    format!("{:<width$} ", name_display, width = max_name_len),
                    style.add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    desc_display,
                    style.fg(self.theme.colors.fg_muted).remove_modifier(Modifier::BOLD),
                ),
            ]);

            buf.set_line(
                results_area.x,
                results_area.y + display_idx as u16,
                &line,
                results_area.width,
            );
        }

        // Footer hint
        let hint = format!(
            " {} of {} ",
            actions.filtered_indices.len(),
            actions.actions.len()
        );
        let hint_len = hint.len() as u16;
        let hint_x = area.x + area.width.saturating_sub(hint_len + 2);
        let hint_y = area.y + area.height - 1;
        let hint_span = Span::styled(hint, Style::default().fg(self.theme.colors.fg_muted));
        buf.set_span(hint_x, hint_y, &hint_span, hint_len);
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else if max_len > 1 {
        format!("{}…", s.chars().take(max_len - 1).collect::<String>())
    } else {
        String::new()
    }
}
