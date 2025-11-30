//! Ports panel widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::core::state::{AppState, FocusedPanel};
use crate::ui::theme::Theme;

pub struct PortsPanel<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> PortsPanel<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl<'a> Widget for PortsPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let focused = self.state.focus_panel == FocusedPanel::Ports;
        let border_style = if focused {
            self.theme.styles.panel_border_focused
        } else {
            self.theme.styles.panel_border
        };

        let block = Block::default()
            .title(Span::styled(" PORT SCOUT ", self.theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(self.theme.colors.bg_primary));

        let inner = block.inner(area);
        block.render(area, buf);

        let ports = &self.state.panels.ports;
        let selected = ports.selected_index;

        // Combine expected and active ports
        let mut display_items: Vec<PortDisplayItem> = Vec::new();

        // Add expected ports with their status
        for expected in &ports.expected_ports {
            let active = ports.active_ports.iter().find(|a| a.port == expected.port);
            let conflict = ports.conflicts.iter().find(|c| c.port == expected.port);

            display_items.push(PortDisplayItem {
                port: expected.port,
                service: expected.service_name.clone(),
                status: if conflict.is_some() {
                    PortStatus::Conflict
                } else if active.is_some() {
                    PortStatus::Active
                } else {
                    PortStatus::Expected
                },
                process: active.map(|a| a.process_name.clone()),
            });
        }

        // Add any active ports not in expected
        for active in &ports.active_ports {
            if !ports.expected_ports.iter().any(|e| e.port == active.port) {
                display_items.push(PortDisplayItem {
                    port: active.port,
                    service: active.process_name.clone(),
                    status: PortStatus::Active,
                    process: Some(active.process_name.clone()),
                });
            }
        }

        // Sort by port number
        display_items.sort_by_key(|i| i.port);

        if display_items.is_empty() {
            let span = Span::styled(
                "No ports detected",
                Style::default().fg(self.theme.colors.fg_muted),
            );
            buf.set_span(inner.x + 1, inner.y, &span, inner.width.saturating_sub(2));
            return;
        }

        for (i, item) in display_items.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let is_selected = i == selected;
            let (icon, icon_style) = match item.status {
                PortStatus::Active => ("✓", self.theme.styles.status_running),
                PortStatus::Expected => ("○", self.theme.styles.list_item),
                PortStatus::Conflict => ("✗", self.theme.styles.status_warning),
            };

            let base_style = if is_selected {
                self.theme.styles.list_item_selected
            } else {
                self.theme.styles.list_item
            };

            let indicator = if is_selected { "▸" } else { " " };
            let service = truncate(&item.service, 12);

            let line = Line::from(vec![
                Span::styled(indicator, base_style),
                Span::styled(
                    format!(" :{:<5} ", item.port),
                    Style::default().fg(self.theme.colors.accent_primary),
                ),
                Span::styled(format!("{} ", icon), icon_style),
                Span::styled(service, base_style),
            ]);

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }
    }
}

struct PortDisplayItem {
    port: u16,
    service: String,
    status: PortStatus,
    #[allow(dead_code)]
    process: Option<String>,
}

enum PortStatus {
    Active,
    Expected,
    Conflict,
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
