//! Configuration system for Orbit

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Global application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub display: DisplayConfig,
    pub keybindings: KeybindingsConfig,
    pub docker: DockerConfig,
    pub focus: FocusConfig,
    pub notifications: NotificationsConfig,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("orbit").join("config.toml"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub check_updates: bool,
    pub startup_time_target: u32,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            check_updates: true,
            startup_time_target: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub theme: String,
    pub layout: String,
    pub animations: bool,
    pub animation_speed: String,
    pub sidebar_width: u16,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            theme: "tokyo-night".to_string(),
            layout: "standard".to_string(),
            animations: true,
            animation_speed: "normal".to_string(),
            sidebar_width: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub quit: String,
    pub palette: String,
    pub focus: String,
    pub toggle_docker: String,
    pub toggle_ports: String,
    pub toggle_env: String,
    pub terminal: String,
    pub help: String,
    pub refresh: String,
    pub navigate_up: String,
    pub navigate_down: String,
    pub select: String,
    pub back: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            palette: "space".to_string(),
            focus: "f".to_string(),
            toggle_docker: "d".to_string(),
            toggle_ports: "p".to_string(),
            toggle_env: "e".to_string(),
            terminal: "t".to_string(),
            help: "?".to_string(),
            refresh: "r".to_string(),
            navigate_up: "k".to_string(),
            navigate_down: "j".to_string(),
            select: "enter".to_string(),
            back: "esc".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DockerConfig {
    pub socket: Option<String>,
    pub stats_interval: u64,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            socket: None,
            stats_interval: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FocusConfig {
    pub default_duration: u32,
    pub enable_dnd: bool,
    pub minimize_windows: bool,
    pub ambient_sound: String,
    pub ambient_volume: u8,
}

impl Default for FocusConfig {
    fn default() -> Self {
        Self {
            default_duration: 25,
            enable_dnd: true,
            minimize_windows: true,
            ambient_sound: "lofi".to_string(),
            ambient_volume: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationsConfig {
    pub native: bool,
    pub on_action_complete: bool,
    pub on_focus_end: bool,
    pub on_port_conflict: bool,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            native: true,
            on_action_complete: true,
            on_focus_end: true,
            on_port_conflict: true,
        }
    }
}

/// Project-specific configuration (.orbit.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    pub display: ProjectDisplayConfig,
    pub actions: ProjectActionsConfig,
    pub secrets: ProjectSecretsConfig,
    pub ports: ProjectPortsConfig,
    pub focus: ProjectFocusConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectDisplayConfig {
    pub docker_panel: bool,
    pub ports_panel: bool,
    pub env_panel: bool,
    pub output_expanded: bool,
    pub layout: String,
    pub theme: Option<String>,
}

impl Default for ProjectDisplayConfig {
    fn default() -> Self {
        Self {
            docker_panel: true,
            ports_panel: true,
            env_panel: true,
            output_expanded: false,
            layout: "standard".to_string(),
            theme: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectActionsConfig {
    pub custom: Vec<CustomAction>,
    pub favorites: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAction {
    pub name: String,
    pub command: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub keybinding: Option<String>,
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectSecretsConfig {
    pub keychain: Vec<String>,
    pub allow_dotenv: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectPortsConfig {
    pub expected: Vec<ExpectedPortConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedPortConfig {
    pub port: u16,
    pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectFocusConfig {
    pub default_duration: Option<u32>,
    pub ambient_sound: Option<String>,
    pub enable_dnd: Option<bool>,
    pub minimize_windows: Option<bool>,
}

impl ProjectConfig {
    pub fn load(dir: &Path) -> Result<Option<Self>> {
        let path = dir.join(".orbit.toml");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(Some(config))
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        let path = dir.join(".orbit.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Initialize a new project configuration
pub fn init_project_config(dir: &Path, force: bool) -> Result<()> {
    let path = dir.join(".orbit.toml");
    
    if path.exists() && !force {
        anyhow::bail!("Configuration already exists. Use --force to overwrite.");
    }

    let project_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    let config = ProjectConfig {
        project: ProjectInfo {
            name: project_name,
            description: String::new(),
        },
        ..Default::default()
    };

    config.save(dir)?;
    println!("Created .orbit.toml");
    Ok(())
}
