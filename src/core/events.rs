//! Unified event handling system

#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::core::state::Notification;

/// All possible events in the system
#[derive(Debug, Clone)]
pub enum Event {
    // Input events
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),

    // System events
    Tick,     // Render tick (target 60fps)
    SlowTick, // Background refresh (1Hz)

    // Async completion events
    ProcessOutput { id: uuid::Uuid, data: Vec<u8> },
    ProcessExited { id: uuid::Uuid, code: i32 },
    FileChanged(String),

    // Focus mode events
    FocusTimerTick { remaining: u32 },
    FocusModeEnded,

    // Notifications
    ShowNotification(Notification),
    DismissNotification(uuid::Uuid),

    // Project events
    ProjectDetected,
    ProjectReload,

    // System metrics
    SystemMetrics {
        cpu_percent: f32,
        memory_used_mb: u64,
        memory_total_mb: u64,
    },

    // Lifecycle
    Quit,
    ForceRefresh,
}

/// Result of handling an event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    Continue,
    Quit,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    pub fn new() -> (Self, mpsc::UnboundedSender<Event>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { rx, tx: tx.clone() }, tx)
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }

    /// Start all event source tasks
    pub fn spawn_sources(event_tx: mpsc::UnboundedSender<Event>) {
        // Terminal input events
        tokio::spawn(Self::terminal_events(event_tx.clone()));

        // Render tick (33ms = ~30fps for TUI)
        tokio::spawn(Self::tick_events(
            event_tx.clone(),
            Duration::from_millis(33),
            Event::Tick,
        ));

        // Background refresh tick (2 seconds)
        tokio::spawn(Self::tick_events(
            event_tx.clone(),
            Duration::from_secs(2),
            Event::SlowTick,
        ));
    }

    async fn terminal_events(tx: mpsc::UnboundedSender<Event>) {
        use crossterm::event::{self, Event as CrosstermEvent};
        use futures::StreamExt;

        let mut reader = event::EventStream::new();
        while let Some(event_result) = reader.next().await {
            let orbit_event = match event_result {
                Ok(CrosstermEvent::Key(key)) => Event::Key(key),
                Ok(CrosstermEvent::Mouse(mouse)) => Event::Mouse(mouse),
                Ok(CrosstermEvent::Resize(w, h)) => Event::Resize(w, h),
                _ => continue,
            };
            if tx.send(orbit_event).is_err() {
                break;
            }
        }
    }

    async fn tick_events(tx: mpsc::UnboundedSender<Event>, interval: Duration, event: Event) {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            if tx.send(event.clone()).is_err() {
                break;
            }
        }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

/// Key binding helper
pub struct KeyBinding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub fn ctrl(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        event.code == self.key && event.modifiers == self.modifiers
    }
}

/// Standard key bindings
pub struct KeyBindings;

impl KeyBindings {
    pub fn quit() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('q'))
    }

    pub fn quit_alt() -> KeyBinding {
        KeyBinding::ctrl(KeyCode::Char('c'))
    }

    pub fn palette() -> KeyBinding {
        KeyBinding::new(KeyCode::Char(' '))
    }

    pub fn focus() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('f'))
    }

    pub fn docker() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('d'))
    }

    pub fn ports() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('p'))
    }

    pub fn env() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('e'))
    }

    pub fn terminal() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('t'))
    }

    pub fn help() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('?'))
    }

    pub fn escape() -> KeyBinding {
        KeyBinding::new(KeyCode::Esc)
    }

    pub fn enter() -> KeyBinding {
        KeyBinding::new(KeyCode::Enter)
    }

    pub fn tab() -> KeyBinding {
        KeyBinding::new(KeyCode::Tab)
    }

    pub fn backtab() -> KeyBinding {
        KeyBinding {
            key: KeyCode::BackTab,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    pub fn up() -> KeyBinding {
        KeyBinding::new(KeyCode::Up)
    }

    pub fn down() -> KeyBinding {
        KeyBinding::new(KeyCode::Down)
    }

    pub fn vim_up() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('k'))
    }

    pub fn vim_down() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('j'))
    }

    pub fn refresh() -> KeyBinding {
        KeyBinding::new(KeyCode::Char('r'))
    }
}
