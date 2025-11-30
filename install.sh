#!/usr/bin/env bash
#
# Orbit Installation Script
# Builds and installs orbit to your PATH
#
# Usage:
#   ./install.sh          # Install or update orbit
#   ./install.sh --help   # Show help
#   ./install.sh --uninstall  # Remove orbit
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default installation directory
DEFAULT_INSTALL_DIR="$HOME/.local/bin"
INSTALL_DIR="${ORBIT_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
BINARY_NAME="orbit"

print_banner() {
    echo -e "${BLUE}"
    echo "   ____       __    _ __ "
    echo "  / __ \\_____/ /_  (_) /_"
    echo " / / / / ___/ __ \\/ / __/"
    echo "/ /_/ / /  / /_/ / / /_  "
    echo "\\____/_/  /_.___/_/\\__/  "
    echo -e "${NC}"
    echo "The Spatial Dashboard for your Terminal Workflow"
    echo ""
}

print_help() {
    print_banner
    echo "Usage: ./install.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --help, -h        Show this help message"
    echo "  --uninstall       Remove orbit from your system"
    echo "  --dir <path>      Install to a custom directory (default: $DEFAULT_INSTALL_DIR)"
    echo "  --release         Build in release mode (default)"
    echo "  --debug           Build in debug mode"
    echo ""
    echo "Environment variables:"
    echo "  ORBIT_INSTALL_DIR  Override default installation directory"
    echo ""
    echo "Examples:"
    echo "  ./install.sh                    # Install/update orbit"
    echo "  ./install.sh --dir /usr/local/bin  # Install to custom location"
    echo "  ./install.sh --uninstall        # Remove orbit"
    echo ""
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_dependencies() {
    log_info "Checking dependencies..."
    
    # Check for Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo (Rust) is not installed."
        echo ""
        echo "Please install Rust first:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        exit 1
    fi
    
    log_success "Cargo found: $(cargo --version)"
}

get_current_version() {
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        "$INSTALL_DIR/$BINARY_NAME" --version 2>/dev/null | head -1 || echo "unknown"
    else
        echo "not installed"
    fi
}

get_new_version() {
    grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "unknown"
}

build_orbit() {
    local build_mode="$1"
    
    log_info "Building orbit in $build_mode mode..."
    
    if [ "$build_mode" = "release" ]; then
        cargo build --release
        BINARY_PATH="target/release/$BINARY_NAME"
    else
        cargo build
        BINARY_PATH="target/debug/$BINARY_NAME"
    fi
    
    if [ ! -f "$BINARY_PATH" ]; then
        log_error "Build failed - binary not found at $BINARY_PATH"
        exit 1
    fi
    
    log_success "Build complete"
}

install_orbit() {
    log_info "Installing orbit to $INSTALL_DIR..."
    
    # Create installation directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        log_info "Creating directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
    
    # Backup existing binary if it exists
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        local backup_path="$INSTALL_DIR/${BINARY_NAME}.backup"
        log_info "Backing up existing installation to $backup_path"
        cp "$INSTALL_DIR/$BINARY_NAME" "$backup_path"
    fi
    
    # Copy binary
    cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    
    log_success "Installed orbit to $INSTALL_DIR/$BINARY_NAME"
}

check_path() {
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        log_warn "$INSTALL_DIR is not in your PATH"
        echo ""
        echo "Add the following to your shell configuration file:"
        echo ""
        
        # Detect shell and suggest appropriate config file
        local shell_name=$(basename "$SHELL")
        local config_file=""
        
        case "$shell_name" in
            bash)
                config_file="~/.bashrc"
                ;;
            zsh)
                config_file="~/.zshrc"
                ;;
            fish)
                config_file="~/.config/fish/config.fish"
                echo -e "  ${YELLOW}set -gx PATH $INSTALL_DIR \$PATH${NC}"
                echo ""
                echo "Or run:"
                echo -e "  ${YELLOW}echo 'set -gx PATH $INSTALL_DIR \$PATH' >> $config_file${NC}"
                return
                ;;
            *)
                config_file="your shell config file"
                ;;
        esac
        
        echo -e "  ${YELLOW}export PATH=\"$INSTALL_DIR:\$PATH\"${NC}"
        echo ""
        echo "Or run:"
        echo -e "  ${YELLOW}echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> $config_file${NC}"
        echo ""
        echo "Then restart your terminal or run:"
        echo -e "  ${YELLOW}source $config_file${NC}"
    else
        log_success "$INSTALL_DIR is already in your PATH"
    fi
}

uninstall_orbit() {
    log_info "Uninstalling orbit..."
    
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        rm "$INSTALL_DIR/$BINARY_NAME"
        log_success "Removed $INSTALL_DIR/$BINARY_NAME"
    else
        log_warn "orbit is not installed at $INSTALL_DIR/$BINARY_NAME"
    fi
    
    # Remove backup if exists
    if [ -f "$INSTALL_DIR/${BINARY_NAME}.backup" ]; then
        rm "$INSTALL_DIR/${BINARY_NAME}.backup"
        log_info "Removed backup file"
    fi
    
    # Clean up cache and config directories
    local cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/orbit"
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/orbit"
    
    if [ -d "$cache_dir" ]; then
        read -p "Remove cache directory ($cache_dir)? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$cache_dir"
            log_success "Removed cache directory"
        fi
    fi
    
    if [ -d "$config_dir" ]; then
        read -p "Remove config directory ($config_dir)? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$config_dir"
            log_success "Removed config directory"
        fi
    fi
    
    echo ""
    log_success "Orbit has been uninstalled"
}

main() {
    local build_mode="release"
    local action="install"
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                print_help
                exit 0
                ;;
            --uninstall)
                action="uninstall"
                shift
                ;;
            --dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --release)
                build_mode="release"
                shift
                ;;
            --debug)
                build_mode="debug"
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                echo "Run './install.sh --help' for usage"
                exit 1
                ;;
        esac
    done
    
    print_banner
    
    # Handle uninstall
    if [ "$action" = "uninstall" ]; then
        uninstall_orbit
        exit 0
    fi
    
    # Get script directory and change to it
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    cd "$SCRIPT_DIR"
    
    # Check if we're in the orbit project directory
    if [ ! -f "Cargo.toml" ]; then
        log_error "Cargo.toml not found. Please run this script from the orbit project directory."
        exit 1
    fi
    
    # Check for required tools
    check_dependencies
    
    # Show version info
    local current_version=$(get_current_version)
    local new_version=$(get_new_version)
    
    echo ""
    log_info "Current version: $current_version"
    log_info "New version: $new_version"
    echo ""
    
    # Build
    build_orbit "$build_mode"
    
    # Install
    install_orbit
    
    # Check PATH
    echo ""
    check_path
    
    # Final message
    echo ""
    echo "----------------------------------------"
    log_success "Installation complete!"
    echo ""
    echo "Run 'orbit --help' to get started"
    echo "Run 'orbit' in any project directory to launch the dashboard"
    echo ""
}

main "$@"
