//! Docker container panel

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::core::state::{AppState, FocusedPanel};
use crate::integrations::docker::ContainerStatus;
use crate::ui::theme::Theme;

pub struct ContainerPanel<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> ContainerPanel<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }

    fn render_container_row(
        &self,
        container: &crate::integrations::docker::ContainerInfo,
        selected: bool,
        _width: u16,
    ) -> Line<'a> {
        let status_icon = match container.status {
            ContainerStatus::Running => "●",
            ContainerStatus::Paused => "◐",
            ContainerStatus::Restarting => "↻",
            ContainerStatus::Exited => "○",
            ContainerStatus::Dead => "✗",
            ContainerStatus::Created => "◌",
            ContainerStatus::Unknown => "?",
        };

        let status_style = match container.status {
            ContainerStatus::Running => self.theme.styles.status_running,
            ContainerStatus::Exited => self.theme.styles.status_stopped,
            ContainerStatus::Dead => self.theme.styles.status_warning,
            _ => self.theme.styles.list_item,
        };

        let name = truncate(&container.name, 12);
        let (cpu_percent, memory_mb) = container
            .stats
            .as_ref()
            .map(|s| (s.cpu_percent, s.memory_usage_mb as u64))
            .unwrap_or((0.0, 0));
        let cpu = format!("{:>3.0}%", cpu_percent);
        let mem = format!("{:>4}M", memory_mb);

        let base_style = if selected {
            self.theme.styles.list_item_selected
        } else {
            self.theme.styles.list_item
        };

        let indicator = if selected { "▸" } else { " " };

        Line::from(vec![
            Span::styled(indicator, base_style),
            Span::styled(format!(" {} ", status_icon), status_style),
            Span::styled(format!("{:<12} ", name), base_style),
            Span::styled(cpu, base_style.fg(self.theme.colors.accent_primary)),
            Span::raw(" "),
            Span::styled(mem, base_style.fg(self.theme.colors.accent_secondary)),
        ])
    }
}

impl<'a> Widget for ContainerPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let focused = self.state.focus_panel == FocusedPanel::Docker;
        let border_style = if focused {
            self.theme.styles.panel_border_focused
        } else {
            self.theme.styles.panel_border
        };

        let block = Block::default()
            .title(Span::styled(" CONTAINERS ", self.theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(self.theme.colors.bg_primary));

        let inner = block.inner(area);
        block.render(area, buf);

        let containers = &self.state.panels.docker.containers;
        let selected = self.state.panels.docker.selected_index;

        if containers.is_empty() {
            let msg = if self.state.panels.docker.loading {
                "Loading..."
            } else if self.state.panels.docker.error.is_some() {
                "Docker unavailable"
            } else {
                "No containers"
            };
            let span = Span::styled(msg, Style::default().fg(self.theme.colors.fg_muted));
            buf.set_span(inner.x + 1, inner.y, &span, inner.width.saturating_sub(2));
            return;
        }

        for (i, container) in containers.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }
            let line = self.render_container_row(container, i == selected, inner.width);
            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
