//! Main UI renderer

use ratatui::{layout::Rect, Frame};

use crate::core::state::{AppMode, AppState};
use crate::ui::layout::LayoutManager;
use crate::ui::widgets::*;

pub struct Renderer;

impl Renderer {
    pub fn render(frame: &mut Frame, state: &AppState) {
        let area = frame.area();
        let theme = &state.theme;

        // Clear background
        frame.render_widget(
            ratatui::widgets::Block::default()
                .style(ratatui::style::Style::default().bg(theme.colors.bg_primary)),
            area,
        );

        // Compute layout
        let layout = LayoutManager::compute(area, state);

        // Render header
        frame.render_widget(Header::new(state, theme), layout.header);

        // Render footer
        frame.render_widget(Footer::new(state, theme), layout.footer);

        // Render panels based on layout
        if let Some(docker_area) = layout.docker_panel {
            frame.render_widget(ContainerPanel::new(state, theme), docker_area);
        }

        if let Some(ports_area) = layout.ports_panel {
            frame.render_widget(PortsPanel::new(state, theme), ports_area);
        }

        if let Some(system_area) = layout.system_panel {
            frame.render_widget(MetricsPanel::new(state, theme), system_area);
        }

        if let Some(actions_area) = layout.actions_panel {
            frame.render_widget(ActionsPanel::new(state, theme), actions_area);
        }

        if let Some(env_area) = layout.env_panel {
            frame.render_widget(EnvPanel::new(state, theme), env_area);
        }

        if let Some(output_area) = layout.output_panel {
            frame.render_widget(OutputPanel::new(state, theme), output_area);
        }

        // Render overlays based on mode
        match &state.mode {
            AppMode::CommandPalette => {
                if let Some(overlay_area) = layout.overlay_area {
                    frame.render_widget(ActionPalette::new(state, theme), overlay_area);
                }
            }
            AppMode::Help => {
                if let Some(overlay_area) = layout.overlay_area {
                    frame.render_widget(HelpOverlay::new(theme), overlay_area);
                }
            }
            AppMode::FocusMode {
                remaining_seconds,
                ambient_playing,
            } => {
                Self::render_focus_mode(frame, state, *remaining_seconds, *ambient_playing);
            }
            AppMode::Confirm { message, .. } => {
                if let Some(overlay_area) = layout.overlay_area {
                    Self::render_confirm_dialog(frame, state, message, overlay_area);
                }
            }
            _ => {}
        }

        // Render notifications
        Self::render_notifications(frame, state);
    }

    fn render_focus_mode(
        frame: &mut Frame,
        state: &AppState,
        remaining_seconds: u32,
        ambient_playing: bool,
    ) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let area = frame.area();
        let theme = &state.theme;

        // Center the timer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(10),
                Constraint::Percentage(30),
            ])
            .split(area);

        let center_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(chunks[1]);

        let timer_area = center_chunks[1];

        // Format time
        let minutes = remaining_seconds / 60;
        let seconds = remaining_seconds % 60;
        let time_display = format!("{:02}:{:02}", minutes, seconds);

        let block = Block::default()
            .title(Span::styled(" 🎯 Focus Mode ", theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(theme.styles.panel_border_focused)
            .style(Style::default().bg(theme.colors.bg_secondary));

        let inner = block.inner(timer_area);
        frame.render_widget(block, timer_area);

        // Large timer display
        let timer_line = Line::from(vec![Span::styled(
            &time_display,
            Style::default()
                .fg(theme.colors.accent_primary)
                .add_modifier(Modifier::BOLD),
        )]);

        // Center the timer text
        let _x = inner.x + (inner.width.saturating_sub(time_display.len() as u16)) / 2;
        let y = inner.y + inner.height / 2;

        frame.render_widget(
            Paragraph::new(timer_line).alignment(ratatui::layout::Alignment::Center),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );

        // Status line
        let status = if ambient_playing {
            "♪ Ambient playing • Press Esc to exit"
        } else {
            "Press Esc to exit focus mode"
        };

        let status_line = Line::from(vec![Span::styled(
            status,
            Style::default().fg(theme.colors.fg_muted),
        )]);

        frame.render_widget(
            Paragraph::new(status_line).alignment(ratatui::layout::Alignment::Center),
            Rect {
                x: inner.x,
                y: y + 2,
                width: inner.width,
                height: 1,
            },
        );
    }

    fn render_confirm_dialog(frame: &mut Frame, state: &AppState, message: &str, area: Rect) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Clear, Paragraph};

        let theme = &state.theme;

        // Make dialog smaller
        let dialog_area = centered_rect(40, 20, area);

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(" Confirm ", theme.styles.panel_title))
            .borders(Borders::ALL)
            .border_style(theme.styles.panel_border_focused)
            .style(Style::default().bg(theme.colors.bg_secondary));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Message
        let msg_para = Paragraph::new(message)
            .style(Style::default().fg(theme.colors.fg_primary))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(
            msg_para,
            Rect {
                y: inner.y + 1,
                height: 2,
                ..inner
            },
        );

        // Buttons
        let buttons = Line::from(vec![
            Span::styled("[Y]es", theme.styles.keybind_key),
            Span::styled("  ", theme.styles.keybind),
            Span::styled("[N]o", theme.styles.keybind_key),
        ]);

        frame.render_widget(
            Paragraph::new(buttons).alignment(ratatui::layout::Alignment::Center),
            Rect {
                y: inner.y + inner.height - 2,
                height: 1,
                ..inner
            },
        );
    }

    fn render_notifications(frame: &mut Frame, state: &AppState) {
        use ratatui::style::Style;
        use ratatui::text::Span;
        use ratatui::widgets::Paragraph;

        let theme = &state.theme;
        let area = frame.area();

        // Show notifications in top-right corner
        let mut y = 2;
        for notification in state.notifications.iter().take(3) {
            let style = match notification.level {
                crate::core::state::NotificationLevel::Info => theme.styles.notification_info,
                crate::core::state::NotificationLevel::Success => theme.styles.notification_success,
                crate::core::state::NotificationLevel::Warning => theme.styles.status_warning,
                crate::core::state::NotificationLevel::Error => theme.styles.notification_error,
            };

            let icon = match notification.level {
                crate::core::state::NotificationLevel::Info => "ℹ",
                crate::core::state::NotificationLevel::Success => "✓",
                crate::core::state::NotificationLevel::Warning => "⚠",
                crate::core::state::NotificationLevel::Error => "✗",
            };

            let msg = format!(" {} {} ", icon, notification.message);
            let width = (msg.len() as u16).min(40);
            let x = area.width.saturating_sub(width + 2);

            let notification_area = Rect {
                x,
                y,
                width,
                height: 1,
            };

            frame.render_widget(
                Paragraph::new(Span::styled(&msg, style))
                    .style(Style::default().bg(theme.colors.bg_tertiary)),
                notification_area,
            );

            y += 2;
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
