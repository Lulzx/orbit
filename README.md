# Orbit

**The Spatial Dashboard for your Terminal Workflow**

Orbit is a modern terminal user interface (TUI) application designed to streamline your development workflow. It provides a unified dashboard for monitoring Docker containers, managing ports, tracking environment variables, and executing project actions—all from your terminal.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey.svg)

## Features

- **Project Detection** - Automatically detects Node.js, Rust, Python, Go, and Docker projects
- **Docker Integration** - Monitor and manage containers with real-time stats
- **Port Scout** - Track active ports and detect conflicts
- **Environment Management** - View and manage environment variables
- **Secrets Management** - Secure storage via macOS Keychain
- **Focus Mode** - Distraction-free work sessions with Do Not Disturb integration
- **Action Palette** - Quick access to project scripts and commands
- **Real-time Output** - See command output as it streams
- **Beautiful Themes** - Tokyo Night, Catppuccin, Dracula, Nord, and Gruvbox

## Installation

### Quick Install

```bash
# Clone and install
git clone https://github.com/lulzx/orbit.git
cd orbit
./install.sh
```

The install script will:
- Build orbit in release mode
- Install to `~/.local/bin` (or custom location with `--dir`)
- Guide you to add to PATH if needed

### Install Options

```bash
./install.sh                     # Install/update orbit
./install.sh --dir /usr/local/bin   # Custom install location
./install.sh --debug             # Build debug version
./install.sh --uninstall         # Remove orbit
```

### Manual Installation

```bash
cargo build --release
cp target/release/orbit ~/.local/bin/
```

### Requirements

- Rust 1.70 or later
- macOS or Linux
- Docker (optional, for container management)

## Usage

### Interactive TUI

Launch the interactive dashboard in your project directory:

```bash
orbit
```

Or specify a path:

```bash
orbit --path /path/to/project
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Space` | Open command palette |
| `?` | Show help |
| `q` / `Ctrl+C` | Quit |
| `Tab` / `Shift+Tab` | Switch panels |
| `j/k` or `↑/↓` | Navigate |
| `Enter` | Execute action |
| `Esc` | Close palette/dialog |
| `f` | Enter focus mode |
| `d` | Toggle Docker panel |
| `p` | Toggle ports panel |
| `e` | Toggle environment panel |
| `r` | Refresh project |

### Command Palette

Press `Space` to open the command palette where you can:
- Search and filter actions by typing
- Execute project scripts (npm, cargo, make, etc.)
- Toggle panels and settings
- Enter focus mode
- Access system commands

### CLI Commands

```bash
# List detected project actions
orbit actions

# Show all actions including system ones
orbit actions --all

# Show environment variable status
orbit env
orbit env --show-values

# Show port status
orbit ports
orbit ports --kill 3000    # Kill process on port

# Show Docker container status
orbit docker
orbit docker --up          # Start containers
orbit docker --down        # Stop containers

# Enter focus mode (25 minutes with ambient sound)
orbit focus --duration 25 --ambient --sound lofi

# Manage secrets
orbit secrets list
orbit secrets set API_KEY
orbit secrets remove API_KEY
orbit secrets inject --shell zsh

# Initialize project configuration
orbit init
orbit init --force
```

## Configuration

### Global Configuration

Create `~/.config/orbit/config.toml`:

```toml
[general]
check_updates = true
startup_time_target = 50

[display]
theme = "tokyo-night"  # tokyo-night, catppuccin, dracula, nord, gruvbox
layout = "standard"    # standard, compact, wide
animations = true

[keybindings]
quit = "q"
palette = "space"
focus = "f"

[docker]
stats_interval = 2

[focus]
default_duration = 25
enable_dnd = true
minimize_windows = true
ambient_sound = "lofi"
ambient_volume = 30

[notifications]
native = true
on_action_complete = true
on_focus_end = true
on_port_conflict = true
```

### Project Configuration

Create `.orbit.toml` in your project root:

```toml
[project]
name = "my-project"
description = "My awesome project"

[display]
docker_panel = true
ports_panel = true
env_panel = true

[actions]
favorites = ["dev", "build", "test"]

[[actions.custom]]
name = "deploy"
command = "npm run deploy"
category = "deploy"
description = "Deploy to production"
confirm = true

[secrets]
keychain = ["API_KEY", "DATABASE_URL"]

[[ports.expected]]
port = 3000
service = "dev-server"

[[ports.expected]]
port = 5432
service = "postgres"

[focus]
default_duration = 30
ambient_sound = "rain"
```

## Project Detection

Orbit automatically detects and configures support for:

| Project Type | Detection | Scripts From |
|-------------|-----------|--------------|
| **Node.js** | `package.json` | npm/yarn/pnpm/bun scripts |
| **Rust** | `Cargo.toml` | cargo commands |
| **Python** | `pyproject.toml`, `requirements.txt` | scripts, common commands |
| **Go** | `go.mod` | go commands |
| **Docker** | `Dockerfile`, `docker-compose.yml` | compose services |
| **Generic** | `Makefile` | make targets |

### Detected Actions

Orbit discovers runnable commands from:
- `package.json` scripts (dev, build, test, lint, etc.)
- `Makefile` targets
- `Cargo.toml` binaries and examples
- `docker-compose.yml` services
- `pyproject.toml` scripts

## Focus Mode

Focus mode helps you concentrate by:

1. Enabling macOS Do Not Disturb
2. Minimizing other windows (optional)
3. Playing ambient sounds (lofi, rain, cafe, forest, fireplace)
4. Displaying a countdown timer

```bash
# Start a 25-minute focus session
orbit focus

# Custom duration with rain sounds
orbit focus --duration 45 --ambient --sound rain

# No ambient sound
orbit focus --duration 20
```

Press `Esc` or `q` to exit focus mode early.

## Themes

Orbit includes several beautiful color themes:

- **Tokyo Night** (default) - A clean, dark theme with blue accents
- **Catppuccin Mocha** - Warm, pastel colors
- **Dracula** - Classic dark theme with purple accents
- **Nord** - Arctic, bluish color palette
- **Gruvbox** - Retro groove colors

Change themes in your config file.

## Architecture

```
src/
├── main.rs           # CLI entry point
├── actions/          # Action execution system
├── config/           # Configuration management
├── core/
│   ├── app.rs        # Main application loop
│   ├── events.rs     # Event handling
│   └── state.rs      # Application state
├── detection/        # Project detection
│   └── analyzers/    # Language-specific analyzers
├── focus/            # Focus mode implementation
├── integrations/
│   ├── docker/       # Docker integration
│   └── ports/        # Port scanning
├── secrets/          # Keychain integration
└── ui/
    ├── layout/       # Layout management
    ├── theme/        # Color themes
    ├── widgets/      # UI components
    └── renderer.rs   # Main renderer
```

## Development

```bash
# Run in development mode
cargo run

# Run with verbose logging
cargo run -- -vv

# Run tests
cargo test

# Build release
cargo build --release

# Check for issues
cargo clippy

# Format code
cargo fmt
```

## Troubleshooting

### Actions not executing
- Make sure the action has a command (system actions like Quit work differently)
- Check the OUTPUT panel for error messages
- Verify the command works in your terminal first

### Docker panel empty
- Ensure Docker is running
- Check if you have permissions to access Docker socket

### No output showing
- Output appears in the OUTPUT panel (bottom right by default)
- Use Tab to navigate to the Output panel
- Commands run in the background; output streams in real-time

### Keyboard shortcuts not working
- Make sure you're in Dashboard mode (press Esc to exit dialogs)
- Some keys only work when specific panels are focused

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) for the TUI framework
- Inspired by modern developer tools and terminal applications
- Color themes based on popular editor themes
