//! Application state machine with fine-grained reactive updates

#![allow(dead_code)]

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::actions::Action;
use crate::detection::ProjectContext;
use crate::integrations::docker::ContainerInfo;
use crate::integrations::ports::{ActivePort, ExpectedPort, PortConflict};
use crate::ui::theme::Theme;

/// Top-level application mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// Normal dashboard view
    Dashboard,
    /// Command palette is open
    CommandPalette,
    /// Focus mode active
    FocusMode {
        remaining_seconds: u32,
        ambient_playing: bool,
    },
    /// Embedded terminal is active
    Terminal,
    /// Help overlay showing
    Help,
    /// Secret input modal
    SecretInput { key: String },
    /// Confirmation dialog
    Confirm {
        message: String,
        action_id: String,
    },
}

impl Default for AppMode {
    fn default() -> Self {
        Self::Dashboard
    }
}

/// Which panel currently has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPanel {
    #[default]
    Actions,
    Docker,
    Ports,
    Env,
    Output,
}

impl FocusedPanel {
    pub fn next(self) -> Self {
        match self {
            Self::Actions => Self::Docker,
            Self::Docker => Self::Ports,
            Self::Ports => Self::Env,
            Self::Env => Self::Output,
            Self::Output => Self::Actions,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Actions => Self::Output,
            Self::Docker => Self::Actions,
            Self::Ports => Self::Docker,
            Self::Env => Self::Ports,
            Self::Output => Self::Env,
        }
    }
}

/// Docker panel state
#[derive(Debug, Clone, Default)]
pub struct DockerPanelState {
    pub containers: Vec<ContainerInfo>,
    pub expanded: bool,
    pub selected_index: usize,
    pub loading: bool,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
    pub error: Option<String>,
}

/// Port panel state
#[derive(Debug, Clone, Default)]
pub struct PortPanelState {
    pub expected_ports: Vec<ExpectedPort>,
    pub active_ports: Vec<ActivePort>,
    pub conflicts: Vec<PortConflict>,
    pub selected_index: usize,
    pub loading: bool,
}

/// Actions panel state
#[derive(Debug, Clone, Default)]
pub struct ActionPanelState {
    pub actions: Vec<Action>,
    pub selected_index: usize,
    pub filter: String,
    pub filtered_indices: Vec<usize>,
}

impl ActionPanelState {
    pub fn update_filter(&mut self, filter: String) {
        self.filter = filter.clone();
        let filter_lower = filter.to_lowercase();

        if filter.is_empty() {
            self.filtered_indices = (0..self.actions.len()).collect();
        } else {
            self.filtered_indices = self
                .actions
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    a.name.to_lowercase().contains(&filter_lower)
                        || a.command.to_lowercase().contains(&filter_lower)
                        || a.description
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&filter_lower))
                            .unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();
        }

        self.selected_index = 0;
    }

    pub fn selected_action(&self) -> Option<&Action> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.actions.get(idx))
    }
}

/// Environment panel state
#[derive(Debug, Clone, Default)]
pub struct EnvPanelState {
    pub variables: Vec<EnvVariable>,
    pub selected_index: usize,
    pub show_values: bool,
}

#[derive(Debug, Clone)]
pub struct EnvVariable {
    pub name: String,
    pub value: Option<String>,
    pub source: EnvSource,
    pub required: bool,
    pub is_secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvSource {
    Shell,
    DotEnv,
    Keychain,
    Missing,
}

/// System metrics panel state
#[derive(Debug, Clone, Default)]
pub struct MetricsPanelState {
    pub cpu_percent: f32,
    pub cpu_history: VecDeque<f32>,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_percent: f32,
}

impl MetricsPanelState {
    pub fn push_cpu(&mut self, value: f32) {
        self.cpu_history.push_back(value);
        if self.cpu_history.len() > 60 {
            self.cpu_history.pop_front();
        }
        self.cpu_percent = value;
    }
}

/// Output panel state
#[derive(Debug, Clone)]
pub struct OutputPanelState {
    pub lines: VecDeque<OutputLine>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub max_lines: usize,
}

impl Default for OutputPanelState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct OutputLine {
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stream: OutputStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
    System,
}

impl OutputPanelState {
    pub fn new() -> Self {
        Self {
            lines: VecDeque::new(),
            scroll_offset: 0,
            auto_scroll: true,
            max_lines: 1000,
        }
    }

    pub fn push(&mut self, content: String, stream: OutputStream) {
        self.lines.push_back(OutputLine {
            content,
            timestamp: chrono::Utc::now(),
            stream,
        });
        if self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }
        if self.auto_scroll {
            self.scroll_offset = self.lines.len().saturating_sub(1);
        }
    }
}

/// Notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: uuid::Uuid,
    pub message: String,
    pub level: NotificationLevel,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Command history entry
#[derive(Debug, Clone)]
pub struct CommandHistoryEntry {
    pub id: uuid::Uuid,
    pub action_id: String,
    pub command: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub exit_code: Option<i32>,
}

/// Layout configuration
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub preset: LayoutPreset,
    pub docker_panel_visible: bool,
    pub ports_panel_visible: bool,
    pub env_panel_visible: bool,
    pub output_panel_expanded: bool,
    pub sidebar_width_percent: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            preset: LayoutPreset::Standard,
            docker_panel_visible: true,
            ports_panel_visible: true,
            env_panel_visible: true,
            output_panel_expanded: false,
            sidebar_width_percent: 30,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutPreset {
    #[default]
    Standard,
    Compact,
    Wide,
    FocusMode,
    TerminalFocus,
}

/// Granular view state for all panels
#[derive(Debug, Clone, Default)]
pub struct PanelStates {
    pub docker: DockerPanelState,
    pub ports: PortPanelState,
    pub actions: ActionPanelState,
    pub env: EnvPanelState,
    pub metrics: MetricsPanelState,
    pub output: OutputPanelState,
}

/// Main application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub mode: AppMode,
    pub project: Option<ProjectContext>,
    pub panels: PanelStates,
    pub theme: Theme,
    pub layout: LayoutConfig,
    pub notifications: Vec<Notification>,
    pub focus_panel: FocusedPanel,
    pub command_history: Vec<CommandHistoryEntry>,
    pub working_dir: PathBuf,
    pub terminal_size: (u16, u16),
}

impl AppState {
    pub fn new(working_dir: PathBuf, theme: Theme) -> Self {
        Self {
            mode: AppMode::Dashboard,
            project: None,
            panels: PanelStates::default(),
            theme,
            layout: LayoutConfig::default(),
            notifications: Vec::new(),
            focus_panel: FocusedPanel::Actions,
            command_history: Vec::new(),
            working_dir,
            terminal_size: (80, 24),
        }
    }

    pub fn add_notification(&mut self, message: String, level: NotificationLevel) {
        let notification = Notification {
            id: uuid::Uuid::new_v4(),
            message,
            level,
            created_at: chrono::Utc::now(),
            duration_ms: 5000,
        };
        self.notifications.push(notification);
    }

    pub fn remove_expired_notifications(&mut self) {
        let now = chrono::Utc::now();
        self.notifications.retain(|n| {
            let elapsed = now.signed_duration_since(n.created_at).num_milliseconds() as u64;
            elapsed < n.duration_ms
        });
    }
}

/// Reactive state changes via broadcast channel
#[derive(Debug, Clone)]
pub enum StateChange {
    ModeChanged(AppMode),
    ContainersUpdated,
    PortsUpdated,
    MetricsUpdated,
    NotificationAdded(uuid::Uuid),
    NotificationDismissed(uuid::Uuid),
    PanelFocusChanged(FocusedPanel),
    ThemeChanged,
    ProjectReloaded,
    OutputAppended,
    ActionFilterChanged,
}

/// Thread-safe state store
pub struct StateStore {
    state: Arc<RwLock<AppState>>,
    change_tx: broadcast::Sender<StateChange>,
}

impl StateStore {
    pub fn new(initial: AppState) -> Self {
        let (change_tx, _) = broadcast::channel(256);
        Self {
            state: Arc::new(RwLock::new(initial)),
            change_tx,
        }
    }

    /// Subscribe to state changes
    pub fn subscribe(&self) -> broadcast::Receiver<StateChange> {
        self.change_tx.subscribe()
    }

    /// Atomic state mutation with change notification
    pub fn update<F, R>(&self, mutator: F) -> R
    where
        F: FnOnce(&mut AppState) -> (R, Option<StateChange>),
    {
        let mut state = self.state.write();
        let (result, change) = mutator(&mut state);
        if let Some(change) = change {
            let _ = self.change_tx.send(change);
        }
        result
    }

    /// Notify of a state change
    pub fn notify(&self, change: StateChange) {
        let _ = self.change_tx.send(change);
    }

    /// Read current state
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, AppState> {
        self.state.read()
    }

    /// Get a clone of the current state
    pub fn snapshot(&self) -> AppState {
        self.state.read().clone()
    }
}

impl Clone for StateStore {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            change_tx: self.change_tx.clone(),
        }
    }
}
