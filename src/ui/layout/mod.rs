//! Layout management system

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::core::state::{AppState, LayoutConfig, LayoutPreset};

/// Computed layout rects for all panels
#[derive(Debug, Clone, Default)]
pub struct ComputedLayout {
    pub header: Rect,
    pub footer: Rect,
    pub docker_panel: Option<Rect>,
    pub ports_panel: Option<Rect>,
    pub system_panel: Option<Rect>,
    pub actions_panel: Option<Rect>,
    pub env_panel: Option<Rect>,
    pub output_panel: Option<Rect>,
    pub overlay_area: Option<Rect>,
}

pub struct LayoutManager;

impl LayoutManager {
    /// Compute all panel rects based on terminal size and config
    pub fn compute(area: Rect, state: &AppState) -> ComputedLayout {
        let config = &state.layout;
        
        match config.preset {
            LayoutPreset::Standard => Self::standard_layout(area, config),
            LayoutPreset::Compact => Self::compact_layout(area, config),
            LayoutPreset::Wide => Self::wide_layout(area, config),
            LayoutPreset::FocusMode => Self::focus_layout(area),
            LayoutPreset::TerminalFocus => Self::terminal_focus_layout(area),
        }
    }

    fn standard_layout(area: Rect, config: &LayoutConfig) -> ComputedLayout {
        // Main vertical split: header, body, footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Header
                Constraint::Min(10),    // Body
                Constraint::Length(1),  // Footer
            ])
            .split(area);

        let header = main_chunks[0];
        let footer = main_chunks[2];

        // Body: sidebar | main content
        let sidebar_width = config.sidebar_width_percent.clamp(20, 50);
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(sidebar_width),
                Constraint::Min(40),
            ])
            .split(main_chunks[1]);

        // Sidebar panels
        let mut sidebar_constraints = vec![];
        if config.docker_panel_visible {
            sidebar_constraints.push(Constraint::Length(7));
        }
        if config.ports_panel_visible {
            sidebar_constraints.push(Constraint::Length(8));
        }
        sidebar_constraints.push(Constraint::Length(5)); // System metrics
        sidebar_constraints.push(Constraint::Min(0));    // Spacer

        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(sidebar_constraints.clone())
            .split(body_chunks[0]);

        // Main content panels
        let mut main_constraints = vec![Constraint::Length(12)]; // Actions
        if config.env_panel_visible {
            main_constraints.push(Constraint::Length(8));
        }
        main_constraints.push(Constraint::Min(5)); // Output

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(main_constraints.clone())
            .split(body_chunks[1]);

        // Build layout
        let mut layout = ComputedLayout {
            header,
            footer,
            overlay_area: Some(Self::centered_rect(60, 70, area)),
            ..Default::default()
        };

        // Assign sidebar panels
        let mut sidebar_idx = 0;
        if config.docker_panel_visible {
            layout.docker_panel = Some(sidebar_chunks[sidebar_idx]);
            sidebar_idx += 1;
        }
        if config.ports_panel_visible {
            layout.ports_panel = Some(sidebar_chunks[sidebar_idx]);
            sidebar_idx += 1;
        }
        layout.system_panel = Some(sidebar_chunks[sidebar_idx]);

        // Assign main panels
        layout.actions_panel = Some(main_chunks[0]);
        let mut main_idx = 1;
        if config.env_panel_visible {
            layout.env_panel = Some(main_chunks[main_idx]);
            main_idx += 1;
        }
        layout.output_panel = Some(main_chunks[main_idx]);

        layout
    }

    fn compact_layout(area: Rect, _config: &LayoutConfig) -> ComputedLayout {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(area);

        let body_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),
                Constraint::Min(5),
            ])
            .split(main_chunks[1]);

        ComputedLayout {
            header: main_chunks[0],
            footer: main_chunks[2],
            actions_panel: Some(body_chunks[0]),
            output_panel: Some(body_chunks[1]),
            overlay_area: Some(Self::centered_rect(60, 70, area)),
            ..Default::default()
        }
    }

    fn wide_layout(area: Rect, _config: &LayoutConfig) -> ComputedLayout {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(area);

        // Three columns for wide layout
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(main_chunks[1]);

        // Left column: Docker, Ports
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Min(0),
            ])
            .split(body_chunks[0]);

        // Right column: Env, System
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(body_chunks[2]);

        // Middle: Actions, Output
        let middle_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Min(5),
            ])
            .split(body_chunks[1]);

        ComputedLayout {
            header: main_chunks[0],
            footer: main_chunks[2],
            docker_panel: Some(left_chunks[0]),
            ports_panel: Some(left_chunks[1]),
            actions_panel: Some(middle_chunks[0]),
            output_panel: Some(middle_chunks[1]),
            env_panel: Some(right_chunks[0]),
            system_panel: Some(right_chunks[1]),
            overlay_area: Some(Self::centered_rect(50, 60, area)),
        }
    }

    fn focus_layout(area: Rect) -> ComputedLayout {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(12),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        ComputedLayout {
            header: Rect::default(),
            footer: chunks[2],
            // Use 40% width and 50% height for a visible overlay
            overlay_area: Some(Self::centered_rect(40, 50, area)),
            ..Default::default()
        }
    }

    fn terminal_focus_layout(area: Rect) -> ComputedLayout {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(area);

        ComputedLayout {
            header: chunks[0],
            footer: chunks[2],
            output_panel: Some(chunks[1]),
            overlay_area: Some(Self::centered_rect(60, 70, area)),
            ..Default::default()
        }
    }

    /// Create a centered rect with given percentage width/height
    fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
}
