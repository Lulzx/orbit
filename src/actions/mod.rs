//! Actions system - Command execution and management

#![allow(dead_code)]

use anyhow::Result;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::collections::HashMap;
use std::io::{BufRead, BufReader as StdBufReader};
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::detection::{DiscoveredScript, ScriptCategory, ScriptSource};

/// An executable action
#[derive(Debug, Clone)]
pub struct Action {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub category: ActionCategory,
    pub source: ActionSource,
    pub keybinding: Option<String>,
    pub requires_confirm: bool,
    pub env_required: Vec<String>,
    pub working_dir: Option<PathBuf>,
}

impl Action {
    pub fn from_script(script: DiscoveredScript) -> Self {
        Self {
            id: format!("script:{}", script.name),
            name: script.name,
            command: script.command,
            description: script.description,
            category: ActionCategory::from(script.category),
            source: ActionSource::from(script.source),
            keybinding: None,
            requires_confirm: false,
            env_required: script.env_required,
            working_dir: None,
        }
    }

    /// Check if action matches a search query
    pub fn matches(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.name.to_lowercase().contains(&query_lower)
            || self.command.to_lowercase().contains(&query_lower)
            || self
                .description
                .as_ref()
                .map(|d| d.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionCategory {
    Dev,
    Build,
    Test,
    Lint,
    Deploy,
    Database,
    Docker,
    System,
    Custom,
}

impl From<ScriptCategory> for ActionCategory {
    fn from(cat: ScriptCategory) -> Self {
        match cat {
            ScriptCategory::Dev => Self::Dev,
            ScriptCategory::Build => Self::Build,
            ScriptCategory::Test => Self::Test,
            ScriptCategory::Lint => Self::Lint,
            ScriptCategory::Deploy => Self::Deploy,
            ScriptCategory::Database => Self::Database,
            ScriptCategory::Docker => Self::Docker,
            ScriptCategory::Utility => Self::Custom,
        }
    }
}

impl std::fmt::Display for ActionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Build => write!(f, "build"),
            Self::Test => write!(f, "test"),
            Self::Lint => write!(f, "lint"),
            Self::Deploy => write!(f, "deploy"),
            Self::Database => write!(f, "database"),
            Self::Docker => write!(f, "docker"),
            Self::System => write!(f, "system"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionSource {
    PackageJson,
    Makefile,
    CargoToml,
    PyProjectToml,
    DockerCompose,
    OrbitConfig,
    Detected,
    System,
}

impl From<ScriptSource> for ActionSource {
    fn from(source: ScriptSource) -> Self {
        match source {
            ScriptSource::PackageJson => Self::PackageJson,
            ScriptSource::Makefile => Self::Makefile,
            ScriptSource::CargoToml => Self::CargoToml,
            ScriptSource::PyProjectToml => Self::PyProjectToml,
            ScriptSource::DockerCompose => Self::DockerCompose,
            ScriptSource::OrbitConfig => Self::OrbitConfig,
            ScriptSource::Detected => Self::Detected,
        }
    }
}

impl std::fmt::Display for ActionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PackageJson => write!(f, "package.json"),
            Self::Makefile => write!(f, "Makefile"),
            Self::CargoToml => write!(f, "Cargo.toml"),
            Self::PyProjectToml => write!(f, "pyproject.toml"),
            Self::DockerCompose => write!(f, "docker-compose"),
            Self::OrbitConfig => write!(f, ".orbit.toml"),
            Self::Detected => write!(f, "detected"),
            Self::System => write!(f, "system"),
        }
    }
}

/// Action execution result
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub action_id: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub success: bool,
}

/// Output line from action execution
#[derive(Debug, Clone)]
pub enum OutputLine {
    Stdout(String),
    Stderr(String),
    Exit(i32),
}

/// Action executor with streaming output
pub struct ActionExecutor {
    working_dir: PathBuf,
    env_vars: HashMap<String, String>,
}

impl ActionExecutor {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            working_dir,
            env_vars: HashMap::new(),
        }
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    pub fn with_envs(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    /// Execute an action with streaming output using a PTY for real-time output
    pub async fn execute(
        &self,
        action: &Action,
        output_tx: mpsc::Sender<OutputLine>,
    ) -> Result<ActionResult> {
        let start = std::time::Instant::now();

        let working_dir = action
            .working_dir
            .as_ref()
            .unwrap_or(&self.working_dir)
            .clone();

        let command = action.command.clone();
        let action_id = action.id.clone();
        let env_vars = self.env_vars.clone();

        // Run PTY in a blocking task since portable-pty is not async
        let result = tokio::task::spawn_blocking(move || {
            execute_with_pty(&command, &working_dir, &env_vars, output_tx)
        })
        .await??;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ActionResult {
            action_id,
            exit_code: result.0,
            duration_ms,
            success: result.0.map(|c| c == 0).unwrap_or(false),
        })
    }

    /// Execute an action and collect all output
    pub async fn execute_collect(&self, action: &Action) -> Result<(ActionResult, Vec<String>)> {
        let (tx, mut rx) = mpsc::channel(1000);
        let mut output = Vec::new();

        let executor = self.execute(action, tx);

        // Collect output while executing
        let collect = async {
            while let Some(line) = rx.recv().await {
                match line {
                    OutputLine::Stdout(s) | OutputLine::Stderr(s) => {
                        output.push(s);
                    }
                    OutputLine::Exit(_) => {}
                }
            }
        };

        let (result, _) = tokio::join!(executor, collect);

        Ok((result?, output))
    }
}

/// Execute a command with PTY for real-time unbuffered output
fn execute_with_pty(
    command: &str,
    working_dir: &PathBuf,
    env_vars: &HashMap<String, String>,
    output_tx: mpsc::Sender<OutputLine>,
) -> Result<(Option<i32>,)> {
    let pty_system = NativePtySystem::default();

    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(command);
    cmd.cwd(working_dir);

    // Add environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Force colors and unbuffered output
    cmd.env("TERM", "xterm-256color");
    cmd.env("CARGO_TERM_COLOR", "always");
    cmd.env("FORCE_COLOR", "1");
    cmd.env("CLICOLOR_FORCE", "1");
    cmd.env("PYTHONUNBUFFERED", "1");

    let mut child = pair.slave.spawn_command(cmd)?;

    // Drop the slave to close it properly - we only need the master
    drop(pair.slave);

    // Read from the master (which receives all output from the PTY)
    let reader = pair.master.try_clone_reader()?;

    // Spawn a thread to read output so we don't block on wait()
    let output_tx_clone = output_tx.clone();
    let read_thread = std::thread::spawn(move || {
        let buf_reader = StdBufReader::new(reader);
        for line in buf_reader.lines() {
            match line {
                Ok(content) => {
                    // Strip ANSI escape codes for cleaner output display
                    let clean_content = strip_ansi_codes(&content);
                    // Send even if empty to preserve blank lines in output
                    if output_tx_clone
                        .blocking_send(OutputLine::Stdout(if clean_content.is_empty() {
                            content
                        } else {
                            clean_content
                        }))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Wait for the child to exit
    let status = child.wait()?;
    let exit_code = status.exit_code() as i32;

    // Drop the master to signal EOF to the reader thread
    drop(pair.master);

    // Wait for the reader thread to finish
    let _ = read_thread.join();

    let _ = output_tx.blocking_send(OutputLine::Exit(exit_code));

    Ok((Some(exit_code),))
}

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // Skip until we hit a letter (end of escape sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if c == '\r' {
            // Skip carriage returns
        } else {
            result.push(c);
        }
    }

    result
}

/// Built-in system actions
pub fn system_actions() -> Vec<Action> {
    vec![
        Action {
            id: "system:refresh".to_string(),
            name: "Refresh".to_string(),
            command: String::new(),
            description: Some("Refresh project analysis".to_string()),
            category: ActionCategory::System,
            source: ActionSource::System,
            keybinding: Some("r".to_string()),
            requires_confirm: false,
            env_required: vec![],
            working_dir: None,
        },
        Action {
            id: "system:toggle_docker".to_string(),
            name: "Toggle Docker Panel".to_string(),
            command: String::new(),
            description: Some("Show/hide Docker panel".to_string()),
            category: ActionCategory::System,
            source: ActionSource::System,
            keybinding: Some("d".to_string()),
            requires_confirm: false,
            env_required: vec![],
            working_dir: None,
        },
        Action {
            id: "system:toggle_ports".to_string(),
            name: "Toggle Ports Panel".to_string(),
            command: String::new(),
            description: Some("Show/hide ports panel".to_string()),
            category: ActionCategory::System,
            source: ActionSource::System,
            keybinding: Some("p".to_string()),
            requires_confirm: false,
            env_required: vec![],
            working_dir: None,
        },
        Action {
            id: "system:focus_mode".to_string(),
            name: "Focus Mode".to_string(),
            command: String::new(),
            description: Some("Enter focus mode".to_string()),
            category: ActionCategory::System,
            source: ActionSource::System,
            keybinding: Some("f".to_string()),
            requires_confirm: false,
            env_required: vec![],
            working_dir: None,
        },
        Action {
            id: "system:help".to_string(),
            name: "Help".to_string(),
            command: String::new(),
            description: Some("Show help".to_string()),
            category: ActionCategory::System,
            source: ActionSource::System,
            keybinding: Some("?".to_string()),
            requires_confirm: false,
            env_required: vec![],
            working_dir: None,
        },
        Action {
            id: "system:quit".to_string(),
            name: "Quit".to_string(),
            command: String::new(),
            description: Some("Exit Orbit".to_string()),
            category: ActionCategory::System,
            source: ActionSource::System,
            keybinding: Some("q".to_string()),
            requires_confirm: false,
            env_required: vec![],
            working_dir: None,
        },
    ]
}

/// Print actions for CLI output
pub fn print_actions(actions: &[DiscoveredScript], include_system: bool) {
    println!("{:<20} {:<12} {:<15} COMMAND", "NAME", "CATEGORY", "SOURCE");
    println!("{}", "-".repeat(80));

    for script in actions {
        println!(
            "{:<20} {:<12} {:<15} {}",
            truncate(&script.name, 18),
            script.category.to_string(),
            script.source.to_string(),
            truncate(&script.command, 30)
        );
    }

    if include_system {
        println!("\nSystem Actions:");
        for action in system_actions() {
            if let Some(kb) = &action.keybinding {
                println!(
                    "  [{:<3}] {:<20} {}",
                    kb,
                    action.name,
                    action.description.unwrap_or_default()
                );
            }
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.chars().take(max_len).collect()
    }
}

/// Registry of all available actions
pub struct ActionRegistry {
    actions: Vec<Action>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            actions: system_actions(),
        }
    }

    pub fn from_project(project: &crate::detection::ProjectContext) -> Self {
        let mut actions = system_actions();

        // Add project scripts as actions
        for script in &project.scripts {
            actions.push(Action::from_script(script.clone()));
        }

        Self { actions }
    }

    pub fn all_actions(&self) -> Vec<Action> {
        self.actions.clone()
    }

    pub fn get(&self, id: &str) -> Option<Action> {
        self.actions.iter().find(|a| a.id == id).cloned()
    }

    pub fn filter(&self, query: &str) -> Vec<&Action> {
        if query.is_empty() {
            self.actions.iter().collect()
        } else {
            self.actions.iter().filter(|a| a.matches(query)).collect()
        }
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_action_matches() {
        let action = Action {
            id: "test".to_string(),
            name: "Run Tests".to_string(),
            command: "npm test".to_string(),
            description: Some("Execute test suite".to_string()),
            category: ActionCategory::Test,
            source: ActionSource::PackageJson,
            keybinding: None,
            requires_confirm: false,
            env_required: vec![],
            working_dir: None,
        };

        assert!(action.matches("test"));
        assert!(action.matches("npm"));
        assert!(action.matches("suite"));
        assert!(!action.matches("build"));
    }

    #[tokio::test]
    async fn executes_and_streams_output() {
        let executor = ActionExecutor::new(std::env::current_dir().unwrap());
        let action = Action {
            id: "test-echo".to_string(),
            name: "Echo".to_string(),
            command: "printf 'one\\n'; printf 'two\\n' >&2".to_string(),
            description: None,
            category: ActionCategory::Custom,
            source: ActionSource::Detected,
            keybinding: None,
            requires_confirm: false,
            env_required: vec![],
            working_dir: None,
        };

        let (tx, mut rx) = mpsc::channel(16);

        let exec = executor.execute(&action, tx);
        let collector = tokio::spawn(async move {
            let mut lines = Vec::new();
            while let Some(line) = rx.recv().await {
                match line {
                    OutputLine::Stdout(s) | OutputLine::Stderr(s) => lines.push(s),
                    OutputLine::Exit(_) => {}
                }
            }
            lines
        });

        let result = exec.await.expect("execute action");
        let lines = collector.await.expect("collect output");

        assert!(
            result.success,
            "action should succeed: {:?}",
            result.exit_code
        );
        assert!(
            lines.iter().any(|l| l.contains("one")) && lines.iter().any(|l| l.contains("two")),
            "expected to capture stdout and stderr lines, got {:?}",
            lines
        );
    }
}
