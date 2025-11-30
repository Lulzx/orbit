//! Main application orchestrator

#![allow(dead_code)]

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::actions::{ActionExecutor, ActionRegistry};
use crate::config::Config;
use crate::core::events::{Event, EventHandler, EventResult, KeyBindings};
use crate::core::state::{
    AppMode, AppState, FocusedPanel, LayoutPreset, NotificationLevel, OutputStream, StateChange,
    StateStore,
};
use crate::detection::ProjectDetector;
use crate::focus::FocusModeController;
use crate::integrations::docker::DockerClient;
use crate::integrations::system::SystemMonitor;
use crate::ui::renderer::Renderer;
use crate::ui::theme::Theme;

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    state: StateStore,
    event_tx: mpsc::UnboundedSender<Event>,
    docker_client: Option<DockerClient>,
    system_monitor: SystemMonitor,
    action_registry: Arc<ActionRegistry>,
    action_executor: ActionExecutor,
    focus_controller: Option<FocusModeController>,
    config: Config,
    working_dir: PathBuf,
}

impl App {
    pub async fn new(working_dir: PathBuf, config: Config) -> Result<Self> {
        // Initialize terminal
        let backend = CrosstermBackend::new(std::io::stdout());
        let terminal = Terminal::new(backend)?;

        // Load theme
        let theme = Theme::from_name(&config.display.theme);

        // Build initial state
        let initial_state = AppState::new(working_dir.clone(), theme);
        let state = StateStore::new(initial_state);

        // Create a placeholder sender - will be replaced in run()
        let (event_tx, _) = mpsc::unbounded_channel::<Event>();

        // Initialize Docker client if available
        let docker_client = DockerClient::new().ok();
        if docker_client.is_none() {
            tracing::info!("Docker not available");
        }

        // Initialize action registry (will be populated after detection)
        let action_registry = Arc::new(ActionRegistry::new());

        // Initialize action executor
        let action_executor = ActionExecutor::new(working_dir.clone());

        // System metrics collector
        let system_monitor = SystemMonitor::new();

        Ok(Self {
            terminal,
            state,
            event_tx,
            docker_client,
            system_monitor,
            action_registry,
            action_executor,
            focus_controller: None,
            config,
            working_dir,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        self.setup_terminal()?;

        // Run initial project detection
        self.detect_project().await?;

        // Prime background data before first render
        self.refresh_metrics();
        self.refresh_ports().await?;
        self.refresh_docker().await?;

        // Spawn background tasks
        let (mut event_handler, event_tx) = EventHandler::new();
        self.event_tx = event_tx.clone();

        EventHandler::spawn_sources(event_tx.clone());

        // Initial render
        self.render()?;

        // Main event loop
        let result = self.event_loop(&mut event_handler).await;

        // Cleanup
        self.shutdown()?;
        result
    }

    fn setup_terminal(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture,
            crossterm::cursor::Hide,
        )?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Exit focus mode if active
        if let Some(controller) = self.focus_controller.take() {
            let _ = futures::executor::block_on(controller.exit());
        }

        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            crossterm::cursor::Show,
        )?;
        Ok(())
    }

    async fn detect_project(&mut self) -> Result<()> {
        // Show loading state
        self.state.update(|s| {
            s.panels
                .output
                .push("Detecting project...".to_string(), OutputStream::System);
            ((), None)
        });

        // Run detection
        let detector = ProjectDetector::new(&self.working_dir);
        match detector.analyze().await {
            Ok(project) => {
                // Update action registry
                let registry = ActionRegistry::from_project(&project);
                self.action_registry = Arc::new(registry);

                self.state.update(|s| {
                    // Update actions
                    s.panels.actions.actions = self.action_registry.all_actions();
                    s.panels.actions.update_filter(String::new());

                    // Update env vars
                    s.panels.env.variables = project
                        .env_vars
                        .required
                        .iter()
                        .map(|spec| crate::core::state::EnvVariable {
                            name: spec.name.clone(),
                            value: std::env::var(&spec.name).ok(),
                            source: if std::env::var(&spec.name).is_ok() {
                                crate::core::state::EnvSource::Shell
                            } else {
                                crate::core::state::EnvSource::Missing
                            },
                            required: true,
                            is_secret: spec.is_secret,
                        })
                        .collect();

                    // Expected ports for Port Scout
                    s.panels.ports.expected_ports = project
                        .ports
                        .iter()
                        .cloned()
                        .map(crate::integrations::ports::ExpectedPort::from)
                        .collect();

                    // Store project context
                    s.project = Some(project);

                    s.panels.output.push(
                        format!(
                            "Project detected: {} types found",
                            s.project.as_ref().map(|p| p.types.len()).unwrap_or(0)
                        ),
                        OutputStream::System,
                    );

                    ((), Some(crate::core::state::StateChange::ProjectReloaded))
                });
            }
            Err(e) => {
                self.state.update(|s| {
                    s.panels.output.push(
                        format!("Project detection failed: {}", e),
                        OutputStream::System,
                    );
                    ((), None)
                });
            }
        }

        Ok(())
    }

    async fn event_loop(&mut self, event_handler: &mut EventHandler) -> Result<()> {
        loop {
            // Wait for next event
            let Some(event) = event_handler.next().await else {
                break;
            };

            // Handle event
            match self.handle_event(event).await? {
                EventResult::Continue => {}
                EventResult::Quit => break,
            }
        }
        Ok(())
    }

    async fn handle_event(&mut self, event: Event) -> Result<EventResult> {
        match event {
            Event::Key(key) => self.handle_key(key).await,
            Event::Mouse(_mouse) => Ok(EventResult::Continue),
            Event::Resize(w, h) => {
                self.state.update(|s| {
                    s.terminal_size = (w, h);
                    ((), None)
                });
                self.render()?;
                Ok(EventResult::Continue)
            }
            Event::Tick => {
                // Remove expired notifications
                self.state.update(|s| {
                    s.remove_expired_notifications();
                    ((), None)
                });
                self.render()?;
                Ok(EventResult::Continue)
            }
            Event::SlowTick => {
                self.refresh_metrics();
                self.refresh_ports().await?;
                self.refresh_docker().await?;
                Ok(EventResult::Continue)
            }
            Event::FocusTimerTick { remaining } => {
                self.state.update(|s| {
                    if let AppMode::FocusMode {
                        remaining_seconds, ..
                    } = &mut s.mode
                    {
                        *remaining_seconds = remaining;
                    }
                    ((), None)
                });
                Ok(EventResult::Continue)
            }
            Event::FocusModeEnded => {
                self.exit_focus_mode().await?;
                Ok(EventResult::Continue)
            }
            Event::ProjectReload => {
                self.detect_project().await?;
                Ok(EventResult::Continue)
            }
            Event::Quit => Ok(EventResult::Quit),
            Event::ForceRefresh => {
                self.render()?;
                Ok(EventResult::Continue)
            }
            _ => Ok(EventResult::Continue),
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult> {
        let mode = self.state.read().mode.clone();

        match mode {
            AppMode::Dashboard => self.handle_dashboard_key(key).await,
            AppMode::CommandPalette => self.handle_palette_key(key).await,
            AppMode::Help => self.handle_help_key(key),
            AppMode::FocusMode { .. } => self.handle_focus_key(key).await,
            AppMode::Confirm { .. } => self.handle_confirm_key(key).await,
            _ => Ok(EventResult::Continue),
        }
    }

    async fn handle_dashboard_key(&mut self, key: KeyEvent) -> Result<EventResult> {
        // Check for quit
        if KeyBindings::quit().matches(&key) || KeyBindings::quit_alt().matches(&key) {
            return Ok(EventResult::Quit);
        }

        // Mode switches
        if KeyBindings::palette().matches(&key) {
            self.state.update(|s| {
                s.mode = AppMode::CommandPalette;
                s.panels.actions.filter.clear();
                s.panels.actions.update_filter(String::new());
                (
                    (),
                    Some(crate::core::state::StateChange::ModeChanged(
                        AppMode::CommandPalette,
                    )),
                )
            });
            return Ok(EventResult::Continue);
        }

        if KeyBindings::help().matches(&key) {
            self.state.update(|s| {
                s.mode = AppMode::Help;
                (
                    (),
                    Some(crate::core::state::StateChange::ModeChanged(AppMode::Help)),
                )
            });
            return Ok(EventResult::Continue);
        }

        if KeyBindings::focus().matches(&key) {
            self.enter_focus_mode().await?;
            return Ok(EventResult::Continue);
        }

        // Panel toggles
        if KeyBindings::docker().matches(&key) {
            self.state.update(|s| {
                s.layout.docker_panel_visible = !s.layout.docker_panel_visible;
                ((), None)
            });
            return Ok(EventResult::Continue);
        }

        if KeyBindings::ports().matches(&key) {
            self.state.update(|s| {
                s.layout.ports_panel_visible = !s.layout.ports_panel_visible;
                ((), None)
            });
            return Ok(EventResult::Continue);
        }

        if KeyBindings::env().matches(&key) {
            self.state.update(|s| {
                s.layout.env_panel_visible = !s.layout.env_panel_visible;
                ((), None)
            });
            return Ok(EventResult::Continue);
        }

        // Navigation
        if KeyBindings::tab().matches(&key) {
            self.state.update(|s| {
                s.focus_panel = s.focus_panel.next();
                (
                    (),
                    Some(crate::core::state::StateChange::PanelFocusChanged(
                        s.focus_panel,
                    )),
                )
            });
            return Ok(EventResult::Continue);
        }

        if KeyBindings::backtab().matches(&key) {
            self.state.update(|s| {
                s.focus_panel = s.focus_panel.prev();
                (
                    (),
                    Some(crate::core::state::StateChange::PanelFocusChanged(
                        s.focus_panel,
                    )),
                )
            });
            return Ok(EventResult::Continue);
        }

        // Up/Down navigation within focused panel
        if KeyBindings::up().matches(&key) || KeyBindings::vim_up().matches(&key) {
            self.navigate_up();
            return Ok(EventResult::Continue);
        }

        if KeyBindings::down().matches(&key) || KeyBindings::vim_down().matches(&key) {
            self.navigate_down();
            return Ok(EventResult::Continue);
        }

        // Execute selected action
        if KeyBindings::enter().matches(&key) {
            return self.execute_selected_action().await;
        }

        // Refresh
        if KeyBindings::refresh().matches(&key) {
            self.detect_project().await?;
            return Ok(EventResult::Continue);
        }

        Ok(EventResult::Continue)
    }

    async fn handle_palette_key(&mut self, key: KeyEvent) -> Result<EventResult> {
        if KeyBindings::escape().matches(&key) {
            self.state.update(|s| {
                s.mode = AppMode::Dashboard;
                s.panels.actions.filter.clear();
                s.panels.actions.update_filter(String::new());
                (
                    (),
                    Some(crate::core::state::StateChange::ModeChanged(
                        AppMode::Dashboard,
                    )),
                )
            });
            return Ok(EventResult::Continue);
        }

        if KeyBindings::enter().matches(&key) {
            // Execute selected and close palette
            let result = self.execute_selected_action().await?;
            if result == EventResult::Quit {
                return Ok(EventResult::Quit);
            }
            self.state.update(|s| {
                s.mode = AppMode::Dashboard;
                s.panels.actions.filter.clear();
                s.panels.actions.update_filter(String::new());
                (
                    (),
                    Some(crate::core::state::StateChange::ModeChanged(
                        AppMode::Dashboard,
                    )),
                )
            });
            return Ok(EventResult::Continue);
        }

        if KeyBindings::up().matches(&key) || KeyBindings::vim_up().matches(&key) {
            self.state.update(|s| {
                if s.panels.actions.selected_index > 0 {
                    s.panels.actions.selected_index -= 1;
                }
                ((), None)
            });
            return Ok(EventResult::Continue);
        }

        if KeyBindings::down().matches(&key) || KeyBindings::vim_down().matches(&key) {
            self.state.update(|s| {
                let max = s.panels.actions.filtered_indices.len().saturating_sub(1);
                if s.panels.actions.selected_index < max {
                    s.panels.actions.selected_index += 1;
                }
                ((), None)
            });
            return Ok(EventResult::Continue);
        }

        // Text input for filter
        match key.code {
            KeyCode::Char(c) => {
                self.state.update(|s| {
                    let mut filter = s.panels.actions.filter.clone();
                    filter.push(c);
                    s.panels.actions.update_filter(filter);
                    (
                        (),
                        Some(crate::core::state::StateChange::ActionFilterChanged),
                    )
                });
            }
            KeyCode::Backspace => {
                self.state.update(|s| {
                    let mut filter = s.panels.actions.filter.clone();
                    filter.pop();
                    s.panels.actions.update_filter(filter);
                    (
                        (),
                        Some(crate::core::state::StateChange::ActionFilterChanged),
                    )
                });
            }
            _ => {}
        }

        Ok(EventResult::Continue)
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> Result<EventResult> {
        if KeyBindings::escape().matches(&key)
            || KeyBindings::help().matches(&key)
            || KeyBindings::quit().matches(&key)
        {
            self.state.update(|s| {
                s.mode = AppMode::Dashboard;
                (
                    (),
                    Some(crate::core::state::StateChange::ModeChanged(
                        AppMode::Dashboard,
                    )),
                )
            });
        }
        Ok(EventResult::Continue)
    }

    async fn handle_focus_key(&mut self, key: KeyEvent) -> Result<EventResult> {
        if KeyBindings::escape().matches(&key) || KeyBindings::quit().matches(&key) {
            self.exit_focus_mode().await?;
        }
        Ok(EventResult::Continue)
    }

    async fn handle_confirm_key(&mut self, key: KeyEvent) -> Result<EventResult> {
        let action_id = {
            let state = self.state.read();
            if let AppMode::Confirm { action_id, .. } = &state.mode {
                action_id.clone()
            } else {
                return Ok(EventResult::Continue);
            }
        };

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                // Execute the confirmed action
                if let Some(action) = self.action_registry.get(&action_id) {
                    let (tx, _rx) = tokio::sync::mpsc::channel(100);
                    let _ = self.action_executor.execute(&action, tx).await;
                }
                self.state.update(|s| {
                    s.mode = AppMode::Dashboard;
                    (
                        (),
                        Some(crate::core::state::StateChange::ModeChanged(
                            AppMode::Dashboard,
                        )),
                    )
                });
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.update(|s| {
                    s.mode = AppMode::Dashboard;
                    (
                        (),
                        Some(crate::core::state::StateChange::ModeChanged(
                            AppMode::Dashboard,
                        )),
                    )
                });
            }
            _ => {}
        }
        Ok(EventResult::Continue)
    }

    fn navigate_up(&mut self) {
        let focus = self.state.read().focus_panel;
        self.state.update(|s| {
            match focus {
                FocusedPanel::Actions => {
                    if s.panels.actions.selected_index > 0 {
                        s.panels.actions.selected_index -= 1;
                    }
                }
                FocusedPanel::Docker => {
                    if s.panels.docker.selected_index > 0 {
                        s.panels.docker.selected_index -= 1;
                    }
                }
                FocusedPanel::Ports => {
                    if s.panels.ports.selected_index > 0 {
                        s.panels.ports.selected_index -= 1;
                    }
                }
                FocusedPanel::Env => {
                    if s.panels.env.selected_index > 0 {
                        s.panels.env.selected_index -= 1;
                    }
                }
                FocusedPanel::Output => {
                    if s.panels.output.scroll_offset > 0 {
                        s.panels.output.scroll_offset -= 1;
                        s.panels.output.auto_scroll = false;
                    }
                }
            }
            ((), None)
        });
    }

    fn navigate_down(&mut self) {
        let focus = self.state.read().focus_panel;
        self.state.update(|s| {
            match focus {
                FocusedPanel::Actions => {
                    let max = s.panels.actions.filtered_indices.len().saturating_sub(1);
                    if s.panels.actions.selected_index < max {
                        s.panels.actions.selected_index += 1;
                    }
                }
                FocusedPanel::Docker => {
                    let max = s.panels.docker.containers.len().saturating_sub(1);
                    if s.panels.docker.selected_index < max {
                        s.panels.docker.selected_index += 1;
                    }
                }
                FocusedPanel::Ports => {
                    let max = s.panels.ports.active_ports.len().saturating_sub(1);
                    if s.panels.ports.selected_index < max {
                        s.panels.ports.selected_index += 1;
                    }
                }
                FocusedPanel::Env => {
                    let max = s.panels.env.variables.len().saturating_sub(1);
                    if s.panels.env.selected_index < max {
                        s.panels.env.selected_index += 1;
                    }
                }
                FocusedPanel::Output => {
                    let max = s.panels.output.lines.len().saturating_sub(1);
                    if s.panels.output.scroll_offset < max {
                        s.panels.output.scroll_offset += 1;
                    }
                }
            }
            ((), None)
        });
    }

    async fn execute_selected_action(&mut self) -> Result<EventResult> {
        let action = {
            let state = self.state.read();
            state.panels.actions.selected_action().cloned()
        };

        if let Some(action) = action {
            // Handle system actions specially
            if action.id.starts_with("system:") {
                return self.handle_system_action(&action.id).await;
            }

            // Skip if command is empty
            if action.command.is_empty() {
                self.state.update(|s| {
                    s.panels.output.push(
                        format!("Action '{}' has no command to execute", action.name),
                        OutputStream::System,
                    );
                    ((), None)
                });
                let _ = self.event_tx.send(Event::ForceRefresh);
                return Ok(EventResult::Continue);
            }

            self.state.update(|s| {
                s.panels.output.push(
                    format!("Executing: {}", action.command),
                    OutputStream::System,
                );
                ((), None)
            });
            let _ = self.event_tx.send(Event::ForceRefresh);

            // Execute action in a background task so UI remains responsive
            let (tx, mut rx) = tokio::sync::mpsc::channel(100);
            let state = self.state.clone();
            let executor = ActionExecutor::new(self.working_dir.clone());
            let event_tx = self.event_tx.clone();

            // Spawn the entire execution in a background task
            tokio::spawn(async move {
                // Spawn output collector
                let state_for_output = state.clone();
                let event_tx_for_output = event_tx.clone();
                let output_handle = tokio::spawn(async move {
                    while let Some(line) = rx.recv().await {
                        match line {
                            crate::actions::OutputLine::Stdout(s) => {
                                state_for_output.update(|st| {
                                    st.panels.output.push(s, OutputStream::Stdout);
                                    ((), None)
                                });
                                // Request UI refresh
                                let _ = event_tx_for_output.send(Event::ForceRefresh);
                            }
                            crate::actions::OutputLine::Stderr(s) => {
                                state_for_output.update(|st| {
                                    st.panels.output.push(s, OutputStream::Stderr);
                                    ((), None)
                                });
                                // Request UI refresh
                                let _ = event_tx_for_output.send(Event::ForceRefresh);
                            }
                            crate::actions::OutputLine::Exit(_) => {}
                        }
                    }
                });

                // Execute the action
                match executor.execute(&action, tx).await {
                    Ok(result) => {
                        // Wait for output collector to finish
                        let _ = output_handle.await;

                        let msg = if result.success {
                            format!("Completed in {}ms", result.duration_ms)
                        } else {
                            format!("Failed with code {:?}", result.exit_code)
                        };
                        let level = if result.success {
                            NotificationLevel::Success
                        } else {
                            NotificationLevel::Error
                        };
                        state.update(|s| {
                            s.panels.output.push(msg.clone(), OutputStream::System);
                            s.add_notification(msg, level);
                            ((), None)
                        });
                        // Request UI refresh for completion message
                        let _ = event_tx.send(Event::ForceRefresh);
                    }
                    Err(e) => {
                        state.update(|s| {
                            s.panels
                                .output
                                .push(format!("Failed to execute: {}", e), OutputStream::System);
                            s.add_notification(
                                format!("Action failed: {}", e),
                                NotificationLevel::Error,
                            );
                            ((), None)
                        });
                        // Request UI refresh for error message
                        let _ = event_tx.send(Event::ForceRefresh);
                    }
                }
            });
        }
        Ok(EventResult::Continue)
    }

    async fn handle_system_action(&mut self, action_id: &str) -> Result<EventResult> {
        match action_id {
            "system:quit" => {
                return Ok(EventResult::Quit);
            }
            "system:refresh" => {
                self.detect_project().await?;
            }
            "system:toggle_docker" => {
                self.state.update(|s| {
                    s.layout.docker_panel_visible = !s.layout.docker_panel_visible;
                    ((), None)
                });
            }
            "system:toggle_ports" => {
                self.state.update(|s| {
                    s.layout.ports_panel_visible = !s.layout.ports_panel_visible;
                    ((), None)
                });
            }
            "system:toggle_env" => {
                self.state.update(|s| {
                    s.layout.env_panel_visible = !s.layout.env_panel_visible;
                    ((), None)
                });
            }
            "system:focus_mode" => {
                self.enter_focus_mode().await?;
            }
            "system:help" => {
                self.state.update(|s| {
                    s.mode = AppMode::Help;
                    (
                        (),
                        Some(crate::core::state::StateChange::ModeChanged(AppMode::Help)),
                    )
                });
            }
            _ => {
                self.state.update(|s| {
                    s.panels.output.push(
                        format!("Unknown system action: {}", action_id),
                        OutputStream::System,
                    );
                    ((), None)
                });
            }
        }
        Ok(EventResult::Continue)
    }

    async fn enter_focus_mode(&mut self) -> Result<()> {
        let config = crate::focus::FocusModeConfig {
            duration_minutes: self.config.focus.default_duration,
            enable_dnd: self.config.focus.enable_dnd,
            minimize_windows: self.config.focus.minimize_windows,
            ambient_sound: if self.config.focus.ambient_sound.is_empty() {
                None
            } else {
                Some(crate::focus::AmbientPreset::from_name(
                    &self.config.focus.ambient_sound,
                ))
            },
        };

        let duration = config.duration_minutes;
        let ambient_enabled = config.ambient_sound.is_some();
        let controller = FocusModeController::enter(config, self.event_tx.clone()).await?;
        self.focus_controller = Some(controller);

        self.state.update(|s| {
            s.mode = AppMode::FocusMode {
                remaining_seconds: if duration == 0 { 0 } else { duration * 60 },
                ambient_playing: ambient_enabled,
            };
            s.layout.preset = LayoutPreset::FocusMode;
            (
                (),
                Some(crate::core::state::StateChange::ModeChanged(s.mode.clone())),
            )
        });

        Ok(())
    }

    async fn exit_focus_mode(&mut self) -> Result<()> {
        if let Some(controller) = self.focus_controller.take() {
            controller.exit().await?;
        }

        self.state.update(|s| {
            s.mode = AppMode::Dashboard;
            s.layout.preset = LayoutPreset::Standard;
            (
                (),
                Some(crate::core::state::StateChange::ModeChanged(
                    AppMode::Dashboard,
                )),
            )
        });

        Ok(())
    }

    fn refresh_metrics(&mut self) {
        let snapshot = self.system_monitor.sample();
        self.state.update(|s| {
            s.panels.metrics.push_cpu(snapshot.cpu_percent);
            s.panels.metrics.memory_used_mb = snapshot.memory_used_mb;
            s.panels.metrics.memory_total_mb = snapshot.memory_total_mb;
            s.panels.metrics.disk_used_percent = snapshot.disk_used_percent;
            ((), Some(StateChange::MetricsUpdated))
        });
    }

    async fn refresh_ports(&mut self) -> Result<()> {
        self.state.update(|s| {
            s.panels.ports.loading = true;
            ((), None)
        });

        let expected = self.state.read().panels.ports.expected_ports.clone();

        match crate::integrations::ports::scan_active_ports().await {
            Ok(active_ports) => {
                let conflicts =
                    crate::integrations::ports::detect_conflicts(&expected, &active_ports);
                self.state.update(|s| {
                    s.panels.ports.active_ports = active_ports;
                    s.panels.ports.conflicts = conflicts;
                    s.panels.ports.loading = false;
                    ((), Some(StateChange::PortsUpdated))
                });
            }
            Err(e) => {
                self.state.update(|s| {
                    s.panels
                        .output
                        .push(format!("Port scan failed: {}", e), OutputStream::System);
                    s.panels.ports.loading = false;
                    ((), None)
                });
            }
        }

        Ok(())
    }

    async fn refresh_docker(&mut self) -> Result<()> {
        let Some(client) = self.docker_client.as_ref() else {
            return Ok(());
        };

        self.state.update(|s| {
            s.panels.docker.loading = true;
            s.panels.docker.error = None;
            ((), None)
        });

        match client.list_containers(true).await {
            Ok(mut containers) => {
                for container in &mut containers {
                    if let Ok(stats) = client.get_stats(&container.id).await {
                        container.stats = Some(stats);
                    }
                }

                self.state.update(|s| {
                    s.panels.docker.containers = containers;
                    s.panels.docker.loading = false;
                    ((), Some(StateChange::ContainersUpdated))
                });
            }
            Err(e) => {
                self.state.update(|s| {
                    s.panels.docker.loading = false;
                    s.panels.docker.error = Some(e.to_string());
                    ((), None)
                });
            }
        }

        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        let state = self.state.snapshot();
        self.terminal.draw(|frame| {
            Renderer::render(frame, &state);
        })?;
        Ok(())
    }
}
