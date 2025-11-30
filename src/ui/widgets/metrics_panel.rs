//! System metrics panel

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Widget},
};

use crate::core::state::AppState;
use crate::ui::theme::Theme;

pub struct MetricsPanel<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> MetricsPanel<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl<'a> Widget for MetricsPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(Span::styled(" SYSTEM ", self.theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(self.theme.styles.panel_border)
            .style(Style::default().bg(self.theme.colors.bg_primary));

        let inner = block.inner(area);
        block.render(area, buf);

        let metrics = &self.state.panels.metrics;

        // CPU line with sparkline
        if inner.height >= 1 {
            let cpu_data: Vec<u64> = metrics
                .cpu_history
                .iter()
                .map(|&v| (v * 100.0) as u64)
                .collect();

            let label = format!("CPU {:>3.0}% ", metrics.cpu_percent);
            let label_span =
                Span::styled(&label, Style::default().fg(self.theme.colors.fg_secondary));
            buf.set_span(inner.x, inner.y, &label_span, label.len() as u16);

            if !cpu_data.is_empty() {
                let sparkline_width = inner.width.saturating_sub(label.len() as u16 + 1);
                if sparkline_width > 0 {
                    let sparkline_area = Rect {
                        x: inner.x + label.len() as u16,
                        y: inner.y,
                        width: sparkline_width,
                        height: 1,
                    };

                    // Draw simple bar representation
                    let bar_width = sparkline_width as usize;
                    let recent: Vec<u64> = cpu_data
                        .iter()
                        .rev()
                        .take(bar_width)
                        .rev()
                        .cloned()
                        .collect();

                    let bar_chars = "▁▂▃▄▅▆▇█";
                    let bar: String = recent
                        .iter()
                        .map(|&v| {
                            let idx = ((v as usize * 7) / 100).min(7);
                            bar_chars.chars().nth(idx).unwrap_or('▁')
                        })
                        .collect();

                    let bar_span = Span::styled(bar, self.theme.styles.sparkline);
                    buf.set_span(
                        sparkline_area.x,
                        sparkline_area.y,
                        &bar_span,
                        sparkline_area.width,
                    );
                }
            }
        }

        // Memory line
        if inner.height >= 2 {
            let mem_percent = if metrics.memory_total_mb > 0 {
                (metrics.memory_used_mb as f32 / metrics.memory_total_mb as f32) * 100.0
            } else {
                0.0
            };

            let mem_bar_width = 12;
            // Clamp to prevent overflow if mem_percent > 100
            let filled = ((mem_percent.min(100.0) / 100.0) * mem_bar_width as f32) as usize;
            let bar: String = "█".repeat(filled) + &"░".repeat(mem_bar_width - filled);

            let mem_label = format!(
                "MEM {} {:>4}G/{:.0}G",
                bar,
                metrics.memory_used_mb / 1024,
                metrics.memory_total_mb / 1024
            );

            let mem_span = Span::styled(
                mem_label,
                Style::default().fg(self.theme.colors.fg_secondary),
            );
            buf.set_span(inner.x, inner.y + 1, &mem_span, inner.width);
        }

        // Disk line (if we have space)
        if inner.height >= 3 {
            let disk_percent = metrics.disk_used_percent;
            let disk_bar_width = 12;
            // Clamp to prevent overflow if disk_percent > 100
            let filled = ((disk_percent.min(100.0) / 100.0) * disk_bar_width as f32) as usize;
            let bar: String = "█".repeat(filled) + &"░".repeat(disk_bar_width - filled);

            let disk_label = format!("DSK {} {:>3.0}%", bar, disk_percent);
            let disk_span = Span::styled(
                disk_label,
                Style::default().fg(self.theme.colors.fg_secondary),
            );
            buf.set_span(inner.x, inner.y + 2, &disk_span, inner.width);
        }
    }
}
