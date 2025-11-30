//! Output panel widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::core::state::{AppState, FocusedPanel, OutputStream};
use crate::ui::theme::Theme;

pub struct OutputPanel<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> OutputPanel<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl<'a> Widget for OutputPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let focused = self.state.focus_panel == FocusedPanel::Output;
        let border_style = if focused {
            self.theme.styles.panel_border_focused
        } else {
            self.theme.styles.panel_border
        };

        let title = if self.state.panels.output.auto_scroll {
            " OUTPUT "
        } else {
            " OUTPUT (scroll) "
        };

        let block = Block::default()
            .title(Span::styled(title, self.theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(self.theme.colors.bg_primary));

        let inner = block.inner(area);
        block.render(area, buf);

        let output = &self.state.panels.output;

        if output.lines.is_empty() {
            let span = Span::styled(
                "Output will appear here...",
                Style::default().fg(self.theme.colors.fg_muted),
            );
            buf.set_span(inner.x + 1, inner.y, &span, inner.width.saturating_sub(2));
            return;
        }

        let visible_lines = inner.height as usize;
        let total_lines = output.lines.len();
        
        let start = if output.auto_scroll {
            total_lines.saturating_sub(visible_lines)
        } else {
            output.scroll_offset.min(total_lines.saturating_sub(visible_lines))
        };

        for (i, line) in output.lines.iter().skip(start).take(visible_lines).enumerate() {
            let style = match line.stream {
                OutputStream::Stdout => Style::default().fg(self.theme.colors.fg_primary),
                OutputStream::Stderr => Style::default().fg(self.theme.colors.error),
                OutputStream::System => Style::default().fg(self.theme.colors.fg_muted),
            };

            let prefix = match line.stream {
                OutputStream::Stdout => "│ ",
                OutputStream::Stderr => "! ",
                OutputStream::System => "● ",
            };

            let prefix_style = match line.stream {
                OutputStream::Stdout => Style::default().fg(self.theme.colors.fg_muted),
                OutputStream::Stderr => Style::default().fg(self.theme.colors.error),
                OutputStream::System => Style::default().fg(self.theme.colors.accent_primary),
            };

            // Truncate line to fit
            let max_content_width = inner.width.saturating_sub(3) as usize;
            let content = truncate(&line.content, max_content_width);

            let display_line = Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(content, style),
            ]);

            buf.set_line(inner.x, inner.y + i as u16, &display_line, inner.width);
        }

        // Show scroll indicator if not at bottom
        if !output.auto_scroll && start + visible_lines < total_lines {
            let indicator = format!(" ↓ {} more ", total_lines - start - visible_lines);
            let indicator_len = indicator.len() as u16;
            let x = inner.x + inner.width.saturating_sub(indicator_len + 1);
            let y = inner.y + inner.height.saturating_sub(1);
            let span = Span::styled(indicator, Style::default().fg(self.theme.colors.fg_muted));
            buf.set_span(x, y, &span, indicator_len);
        }
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
