//! Orbit - The Spatial Dashboard for your Terminal Workflow
//!
//! A context-aware TUI that analyzes your project directory and provides
//! an intelligent dashboard with live monitoring, quick actions, and
//! deep macOS integration.

mod actions;
mod config;
mod core;
mod detection;
mod focus;
mod integrations;
mod secrets;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::core::app::App;

#[derive(Parser)]
#[command(name = "orbit")]
#[command(author = "Orbit Contributors")]
#[command(version = "0.1.0")]
#[command(about = "The Spatial Dashboard for your Terminal Workflow", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Directory to analyze (defaults to current directory)
    #[arg(short, long, value_name = "PATH")]
    path: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Enter focus mode with DND and minimal distractions
    Focus {
        /// Duration in minutes (0 for indefinite)
        #[arg(short, long, default_value = "25")]
        duration: u32,

        /// Play ambient sounds
        #[arg(short, long)]
        ambient: bool,

        /// Ambient sound URL or preset (lofi, rain, cafe)
        #[arg(short, long, default_value = "lofi")]
        sound: String,
    },

    /// List detected scripts and actions
    Actions {
        /// Show all available actions including system ones
        #[arg(short, long)]
        all: bool,
    },

    /// Show environment variables status
    Env {
        /// Show values (redacted by default)
        #[arg(short, long)]
        show_values: bool,
    },

    /// Manage secrets in macOS Keychain
    Secrets {
        #[command(subcommand)]
        command: SecretsCommands,
    },

    /// Show port status for the project
    Ports {
        /// Kill process on specified port
        #[arg(short, long)]
        kill: Option<u16>,
    },

    /// Show Docker container status
    Docker {
        /// Start all containers defined in docker-compose
        #[arg(short, long)]
        up: bool,

        /// Stop all containers
        #[arg(short, long)]
        down: bool,
    },

    /// Initialize Orbit configuration for this project
    Init {
        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum SecretsCommands {
    /// List secrets stored for this project
    List,
    /// Store a secret in the keychain
    Set {
        /// Secret key name
        key: String,
        /// Secret value (will prompt if not provided)
        value: Option<String>,
    },
    /// Remove a secret from the keychain
    Remove {
        /// Secret key name
        key: String,
    },
    /// Inject secrets into current shell session
    Inject {
        /// Export format (bash, zsh, fish)
        #[arg(short, long, default_value = "zsh")]
        shell: String,
    },
}

fn setup_logging(verbosity: u8) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let level = match verbosity {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let filter = EnvFilter::from_default_env().add_directive(level.into());

    // Log to file in debug mode
    let log_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("orbit")
        .join("logs");

    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "orbit.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Keep the guard alive for the duration of the program
    let _logging_guard = setup_logging(cli.verbose)?;

    let working_dir = cli
        .path
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    let config_path = cli.config.or_else(|| {
        let default_config = dirs::config_dir()?.join("orbit").join("config.toml");
        if default_config.exists() {
            Some(default_config)
        } else {
            None
        }
    });

    let config = if let Some(path) = config_path {
        config::Config::load(&path)?
    } else {
        config::Config::default()
    };

    match cli.command {
        Some(Commands::Focus {
            duration,
            ambient,
            sound,
        }) => {
            focus::enter_focus_mode(duration, ambient, &sound).await?;
        }
        Some(Commands::Actions { all }) => {
            let detector = detection::ProjectDetector::new(&working_dir);
            let context = detector.analyze().await?;
            actions::print_actions(&context.scripts, all);
        }
        Some(Commands::Env { show_values }) => {
            let detector = detection::ProjectDetector::new(&working_dir);
            secrets::print_env_status(&detector, show_values).await?;
        }
        Some(Commands::Secrets { command }) => match command {
            SecretsCommands::List => {
                secrets::list_secrets(&working_dir).await?;
            }
            SecretsCommands::Set { key, value } => {
                secrets::set_secret(&working_dir, &key, value).await?;
            }
            SecretsCommands::Remove { key } => {
                secrets::remove_secret(&working_dir, &key).await?;
            }
            SecretsCommands::Inject { shell } => {
                secrets::inject_secrets(&working_dir, &shell).await?;
            }
        },
        Some(Commands::Ports { kill }) => {
            if let Some(port) = kill {
                integrations::ports::kill_port(port).await?;
            } else {
                let detector = detection::ProjectDetector::new(&working_dir);
                integrations::ports::print_port_status(&detector).await?;
            }
        }
        Some(Commands::Docker { up, down }) => {
            if up {
                integrations::docker::compose_up(&working_dir).await?;
            } else if down {
                integrations::docker::compose_down(&working_dir).await?;
            } else {
                integrations::docker::print_status(&working_dir).await?;
            }
        }
        Some(Commands::Init { force }) => {
            config::init_project_config(&working_dir, force)?;
        }
        None => {
            // Launch the main TUI
            let mut app = App::new(working_dir, config).await?;
            app.run().await?;
        }
    }

    Ok(())
}
