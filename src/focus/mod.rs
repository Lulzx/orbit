//! Focus mode - Distraction-free work sessions with macOS integration

#![allow(dead_code)]

use anyhow::Result;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use crate::core::events::Event;

/// Ambient sound presets
#[derive(Debug, Clone)]
pub enum AmbientSound {
    Lofi,
    Rain,
    Cafe,
    Forest,
    Fireplace,
    Custom(String),
}

/// Alias for AmbientSound
pub type AmbientPreset = AmbientSound;

impl AmbientSound {
    /// Get the URL for the ambient sound
    pub fn url(&self) -> &str {
        match self {
            Self::Lofi => "https://www.youtube.com/watch?v=jfKfPfyJRdk",
            Self::Rain => "https://www.youtube.com/watch?v=mPZkdNFkNps",
            Self::Cafe => "https://www.youtube.com/watch?v=h2zkV-l_TbY",
            Self::Forest => "https://www.youtube.com/watch?v=xNN7iTA57jM",
            Self::Fireplace => "https://www.youtube.com/watch?v=L_LUpnjgPso",
            Self::Custom(url) => url,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lofi" | "lo-fi" => Self::Lofi,
            "rain" => Self::Rain,
            "cafe" | "coffee" => Self::Cafe,
            "forest" | "nature" => Self::Forest,
            "fireplace" | "fire" => Self::Fireplace,
            url if url.starts_with("http") => Self::Custom(url.to_string()),
            _ => Self::Lofi,
        }
    }

    pub fn from_name(name: &str) -> Self {
        Self::from_str(name)
    }
}

/// Configuration for focus mode
#[derive(Debug, Clone)]
pub struct FocusModeConfig {
    pub duration_minutes: u32,
    pub enable_dnd: bool,
    pub minimize_windows: bool,
    pub ambient_sound: Option<AmbientSound>,
}

impl Default for FocusModeConfig {
    fn default() -> Self {
        Self {
            duration_minutes: 25,
            enable_dnd: true,
            minimize_windows: true,
            ambient_sound: None,
        }
    }
}

/// Focus mode controller for TUI integration
pub struct FocusModeController {
    session: FocusSession,
    config: FocusModeConfig,
    event_tx: mpsc::UnboundedSender<Event>,
    cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl FocusModeController {
    pub async fn enter(
        config: FocusModeConfig,
        event_tx: mpsc::UnboundedSender<Event>,
    ) -> Result<Self> {
        // Enable DND if configured
        if config.enable_dnd {
            let _ = enable_dnd().await;
        }

        // Minimize windows if configured
        if config.minimize_windows {
            let _ = minimize_windows().await;
        }

        // Start ambient sound if configured
        if let Some(ref sound) = config.ambient_sound {
            let url = sound.url().to_string();
            tokio::spawn(async move {
                let _ = Command::new("open").arg(&url).output().await;
            });
        }

        let session = FocusSession::new(config.duration_minutes);
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

        // Spawn timer task
        let tx = event_tx.clone();
        let duration = config.duration_minutes;
        tokio::spawn(async move {
            Self::timer_task(tx, duration, cancel_rx).await;
        });

        Ok(Self {
            session,
            config,
            event_tx,
            cancel_tx: Some(cancel_tx),
        })
    }

    async fn timer_task(
        event_tx: mpsc::UnboundedSender<Event>,
        duration_minutes: u32,
        mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
    ) {
        let total_seconds = duration_minutes.saturating_mul(60);
        let indefinite = total_seconds == 0;
        let mut elapsed = 0u32;
        let mut ticker = interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    elapsed += 1;
                    let remaining = if indefinite {
                        0
                    } else {
                        total_seconds.saturating_sub(elapsed)
                    };

                    let _ = event_tx.send(Event::FocusTimerTick { remaining });

                    if !indefinite && elapsed >= total_seconds {
                        let _ = event_tx.send(Event::FocusModeEnded);
                        break;
                    }
                }
                _ = &mut cancel_rx => {
                    break;
                }
            }
        }
    }

    pub async fn exit(mut self) -> Result<()> {
        // Cancel the timer
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }

        // Disable DND
        if self.config.enable_dnd {
            let _ = disable_dnd().await;
        }

        // Stop ambient sound
        let _ = stop_ambient_sound().await;

        Ok(())
    }

    pub fn remaining_seconds(&self) -> u32 {
        self.session.remaining_seconds()
    }
}

/// Focus session state
#[derive(Debug, Clone)]
pub struct FocusSession {
    pub duration_minutes: u32,
    pub elapsed_seconds: u32,
    pub ambient_playing: bool,
    pub dnd_enabled: bool,
    pub paused: bool,
}

impl FocusSession {
    pub fn new(duration_minutes: u32) -> Self {
        Self {
            duration_minutes,
            elapsed_seconds: 0,
            ambient_playing: false,
            dnd_enabled: false,
            paused: false,
        }
    }

    pub fn remaining_seconds(&self) -> u32 {
        let total_seconds = self.duration_minutes * 60;
        total_seconds.saturating_sub(self.elapsed_seconds)
    }

    pub fn progress_percent(&self) -> f32 {
        if self.duration_minutes == 0 {
            return 0.0;
        }
        let total_seconds = self.duration_minutes * 60;
        (self.elapsed_seconds as f32 / total_seconds as f32) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        self.duration_minutes > 0 && self.elapsed_seconds >= self.duration_minutes * 60
    }

    pub fn format_remaining(&self) -> String {
        if self.duration_minutes == 0 {
            return "--:--".to_string();
        }

        let remaining = self.remaining_seconds();
        let minutes = remaining / 60;
        let seconds = remaining % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Focus mode events
#[derive(Debug, Clone)]
pub enum FocusEvent {
    Tick,
    Complete,
    Paused,
    Resumed,
    Cancelled,
}

/// Enter focus mode (CLI command)
pub async fn enter_focus_mode(duration: u32, ambient: bool, sound: &str) -> Result<()> {
    println!("Starting focus mode for {} minutes...", duration);

    // Enable Do Not Disturb
    if let Err(e) = enable_dnd().await {
        eprintln!("Warning: Could not enable DND: {}", e);
    } else {
        println!("Do Not Disturb enabled");
    }

    // Minimize other windows
    if let Err(e) = minimize_windows().await {
        eprintln!("Warning: Could not minimize windows: {}", e);
    }

    // Start ambient sound
    if ambient {
        let sound_preset = AmbientSound::from_str(sound);
        if let Err(e) = play_ambient_sound(&sound_preset).await {
            eprintln!("Warning: Could not start ambient sound: {}", e);
        } else {
            println!("Playing ambient sound: {}", sound);
        }
    }

    // Run timer
    let mut session = FocusSession::new(duration);
    let mut ticker = interval(Duration::from_secs(1));

    println!("\nFocus session started. Press Ctrl+C to end early.\n");

    loop {
        ticker.tick().await;

        if session.paused {
            continue;
        }

        session.elapsed_seconds += 1;

        // Update display every minute
        if session.elapsed_seconds % 60 == 0 {
            let remaining = session.remaining_seconds() / 60;
            println!("{} minutes remaining...", remaining);
        }

        if session.is_complete() {
            break;
        }
    }

    // Session complete
    println!("\nFocus session complete!");

    // Send notification
    send_notification("Focus Session Complete", "Great work! Time for a break.").await?;

    // Disable DND
    if let Err(e) = disable_dnd().await {
        eprintln!("Warning: Could not disable DND: {}", e);
    }

    // Stop ambient sound (if playing)
    if ambient {
        stop_ambient_sound().await?;
    }

    Ok(())
}

/// Enable macOS Do Not Disturb
pub async fn enable_dnd() -> Result<()> {
    // macOS Monterey+ uses Focus system
    // AppleScript approach (commented out, using defaults write instead):
    // tell application "System Events"
    //     tell process "ControlCenter"
    //         click menu bar item "Control Center" of menu bar 1
    //         ...
    //     end tell
    // end tell

    // Use defaults write (more reliable)
    let _ = Command::new("defaults")
        .args([
            "-currentHost",
            "write",
            "com.apple.notificationcenterui",
            "doNotDisturb",
            "-boolean",
            "true",
        ])
        .output()
        .await?;

    // Restart NotificationCenter to apply
    let _ = Command::new("killall")
        .args(["NotificationCenter"])
        .output()
        .await;

    Ok(())
}

/// Disable macOS Do Not Disturb
pub async fn disable_dnd() -> Result<()> {
    let _ = Command::new("defaults")
        .args([
            "-currentHost",
            "write",
            "com.apple.notificationcenterui",
            "doNotDisturb",
            "-boolean",
            "false",
        ])
        .output()
        .await?;

    // Restart NotificationCenter to apply
    let _ = Command::new("killall")
        .args(["NotificationCenter"])
        .output()
        .await;

    Ok(())
}

/// Minimize all windows except Terminal
pub async fn minimize_windows() -> Result<()> {
    let script = r#"
        tell application "System Events"
            set frontApp to name of first application process whose frontmost is true
            set visibleApps to every application process whose visible is true
            repeat with theApp in visibleApps
                set appName to name of theApp
                if appName is not frontApp and appName is not "Finder" and appName is not "Terminal" and appName is not "iTerm2" and appName is not "Alacritty" and appName is not "kitty" and appName is not "WezTerm" then
                    try
                        tell application appName to set miniaturized of every window to true
                    end try
                end if
            end repeat
        end tell
    "#;

    run_applescript(script).await?;
    Ok(())
}

/// Play ambient sound
pub async fn play_ambient_sound(sound: &AmbientSound) -> Result<()> {
    let url = sound.url().to_string();

    // Try to use mpv if available (for YouTube)
    if is_command_available("mpv").await {
        let url_clone = url.clone();
        tokio::spawn(async move {
            let _ = Command::new("mpv")
                .args(["--no-video", "--really-quiet", "--volume=30", &url_clone])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        });
        return Ok(());
    }

    // Fallback: Open in browser
    Command::new("open").arg(&url).output().await?;

    Ok(())
}

/// Stop ambient sound
pub async fn stop_ambient_sound() -> Result<()> {
    // Kill mpv if running
    let _ = Command::new("pkill")
        .args(["-f", "mpv.*youtube"])
        .output()
        .await;

    Ok(())
}

/// Send macOS notification
pub async fn send_notification(title: &str, message: &str) -> Result<()> {
    let script = format!(
        r#"display notification "{}" with title "{}""#,
        message.replace('"', "\\\""),
        title.replace('"', "\\\"")
    );

    run_applescript(&script).await?;
    Ok(())
}

/// Play system sound
pub async fn play_sound(sound: &str) -> Result<()> {
    let sound_file = match sound {
        "complete" | "done" => "/System/Library/Sounds/Glass.aiff",
        "alert" => "/System/Library/Sounds/Ping.aiff",
        "start" => "/System/Library/Sounds/Pop.aiff",
        path if path.starts_with('/') => path,
        _ => "/System/Library/Sounds/Glass.aiff",
    };

    Command::new("afplay").arg(sound_file).output().await?;

    Ok(())
}

/// Run an AppleScript
async fn run_applescript(script: &str) -> Result<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("AppleScript failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check if a command is available
async fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Set system volume (0-100)
pub async fn set_volume(level: u8) -> Result<()> {
    let level = level.min(100);
    let script = format!("set volume output volume {}", level);
    run_applescript(&script).await?;
    Ok(())
}

/// Get current system volume (0-100)
pub async fn get_volume() -> Result<u8> {
    let output = run_applescript("output volume of (get volume settings)").await?;
    let volume: u8 = output.trim().parse().unwrap_or(50);
    Ok(volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_session() {
        let mut session = FocusSession::new(25);
        assert_eq!(session.remaining_seconds(), 1500);
        assert_eq!(session.progress_percent(), 0.0);

        session.elapsed_seconds = 750;
        assert_eq!(session.remaining_seconds(), 750);
        assert!((session.progress_percent() - 50.0).abs() < 0.1);

        session.elapsed_seconds = 1500;
        assert!(session.is_complete());
    }

    #[test]
    fn test_format_remaining() {
        let mut session = FocusSession::new(25);
        session.elapsed_seconds = 0;
        assert_eq!(session.format_remaining(), "25:00");

        session.elapsed_seconds = 1425;
        assert_eq!(session.format_remaining(), "01:15");
    }

    #[test]
    fn test_format_remaining_indefinite() {
        let session = FocusSession::new(0);
        assert_eq!(session.format_remaining(), "--:--");
    }

    #[test]
    fn test_ambient_sound_from_str() {
        assert!(matches!(AmbientSound::from_str("lofi"), AmbientSound::Lofi));
        assert!(matches!(AmbientSound::from_str("rain"), AmbientSound::Rain));
        assert!(matches!(
            AmbientSound::from_str("https://example.com"),
            AmbientSound::Custom(_)
        ));
    }
}
