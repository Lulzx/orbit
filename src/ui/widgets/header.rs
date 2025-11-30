//! Header widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::core::state::AppState;
use crate::ui::theme::Theme;

pub struct Header<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> Header<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl<'a> Widget for Header<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Background
        buf.set_style(area, self.theme.styles.header);

        let project_name = self
            .state
            .project
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("No project");

        // Git info
        let git_info = self
            .state
            .project
            .as_ref()
            .and_then(|p| p.git_info.as_ref())
            .map(|g| {
                let mut parts = vec![g.branch.clone()];
                if g.ahead > 0 {
                    parts.push(format!("↑{}", g.ahead));
                }
                if g.behind > 0 {
                    parts.push(format!("↓{}", g.behind));
                }
                if g.dirty {
                    parts.push("*".to_string());
                }
                parts.join(" ")
            })
            .unwrap_or_default();

        // Project types
        let types: Vec<String> = self
            .state
            .project
            .as_ref()
            .map(|p| {
                p.types
                    .iter()
                    .filter(|t| t.primary)
                    .map(|t| format_project_kind(&t.kind))
                    .collect()
            })
            .unwrap_or_default();
        let types_str = types.join(", ");

        // Docker status
        let docker_status = {
            let containers = &self.state.panels.docker.containers;
            let running = containers
                .iter()
                .filter(|c| c.status == crate::integrations::docker::ContainerStatus::Running)
                .count();
            let total = containers.len();
            if total > 0 {
                format!("🐳 {}/{}", running, total)
            } else {
                String::new()
            }
        };

        // Port status
        let port_status = {
            let conflicts = self.state.panels.ports.conflicts.len();
            let active = self.state.panels.ports.active_ports.len();
            if conflicts > 0 {
                format!("⚡ {} ⚠", active)
            } else if active > 0 {
                format!("⚡ {} ✓", active)
            } else {
                String::new()
            }
        };

        // Time
        let time = chrono::Local::now().format("%H:%M").to_string();

        // Build header line
        let mut spans = vec![
            Span::styled(
                " 🛰️ ORBIT ",
                Style::default()
                    .fg(self.theme.colors.accent_primary)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(self.theme.colors.fg_muted)),
            Span::styled(
                project_name,
                Style::default().fg(self.theme.colors.fg_primary),
            ),
        ];

        if !git_info.is_empty() {
            spans.push(Span::styled(
                " │ ",
                Style::default().fg(self.theme.colors.fg_muted),
            ));
            spans.push(Span::styled(
                git_info,
                Style::default().fg(self.theme.colors.accent_secondary),
            ));
        }

        if !types_str.is_empty() {
            spans.push(Span::styled(
                " │ ",
                Style::default().fg(self.theme.colors.fg_muted),
            ));
            spans.push(Span::styled(
                types_str,
                Style::default().fg(self.theme.colors.info),
            ));
        }

        if !docker_status.is_empty() {
            spans.push(Span::styled(
                " │ ",
                Style::default().fg(self.theme.colors.fg_muted),
            ));
            spans.push(Span::styled(
                docker_status,
                Style::default().fg(self.theme.colors.success),
            ));
        }

        if !port_status.is_empty() {
            spans.push(Span::styled(
                " │ ",
                Style::default().fg(self.theme.colors.fg_muted),
            ));
            spans.push(Span::styled(
                port_status,
                Style::default().fg(self.theme.colors.info),
            ));
        }

        // Calculate right side position for time
        let left_line = Line::from(spans);
        let _left_width = left_line.width();

        buf.set_line(area.x, area.y, &left_line, area.width);

        // Render time on the right
        let time_span = Span::styled(&time, Style::default().fg(self.theme.colors.fg_muted));
        let time_x = area.x + area.width.saturating_sub(time.len() as u16 + 1);
        buf.set_span(time_x, area.y, &time_span, time.len() as u16);
    }
}

fn format_project_kind(kind: &crate::detection::ProjectKind) -> String {
    use crate::detection::ProjectKind;
    match kind {
        ProjectKind::Node { framework, .. } => {
            if let Some(fw) = framework {
                format!("{:?}", fw)
            } else {
                "Node".to_string()
            }
        }
        ProjectKind::Rust { .. } => "Rust".to_string(),
        ProjectKind::Python { framework, .. } => {
            if let Some(fw) = framework {
                format!("{:?}", fw)
            } else {
                "Python".to_string()
            }
        }
        ProjectKind::Go { .. } => "Go".to_string(),
        ProjectKind::Docker { .. } => "Docker".to_string(),
        ProjectKind::Git => "Git".to_string(),
        ProjectKind::Generic => "Generic".to_string(),
    }
}
