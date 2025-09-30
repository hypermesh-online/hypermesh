#!/usr/bin/env bash

# Phoenix SDK Installation Script
# https://phoenix.dev
#
# This script installs the Phoenix CLI and SDK on your system.
# It detects your OS and architecture, downloads the appropriate binary,
# and sets up your development environment.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

# Phoenix configuration
PHOENIX_VERSION="${PHOENIX_VERSION:-1.0.0}"
PHOENIX_BASE_URL="https://releases.phoenix.dev"
PHOENIX_INSTALL_DIR="${PHOENIX_INSTALL_DIR:-$HOME/.phoenix}"
PHOENIX_BIN_DIR="$PHOENIX_INSTALL_DIR/bin"

# Print colored output
print_msg() {
    echo -e "${2}${1}${RESET}"
}

print_success() {
    echo -e "${GREEN}✓${RESET} $1"
}

print_error() {
    echo -e "${RED}✗${RESET} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${RESET} $1"
}

print_info() {
    echo -e "${CYAN}ℹ${RESET} $1"
}

# Print Phoenix banner
print_banner() {
    echo -e "${CYAN}"
    cat << "EOF"
    ____  __                    _
   / __ \/ /_  ____  ___  ____  (_)  __
  / /_/ / __ \/ __ \/ _ \/ __ \/ / |/_/
 / ____/ / / / /_/ /  __/ / / / />  <
/_/   /_/ /_/\____/\___/_/ /_/_/_/|_|

EOF
    echo -e "${BOLD}High-performance distributed computing made simple${RESET}"
    echo ""
}

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux*)
            PLATFORM="linux"
            ;;
        Darwin*)
            PLATFORM="macos"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            PLATFORM="windows"
            ;;
        *)
            print_error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    print_info "Detected platform: $PLATFORM-$ARCH"
}

# Check for required dependencies
check_dependencies() {
    print_msg "Checking dependencies..." "$BOLD"

    # Check for curl or wget
    if command -v curl >/dev/null 2>&1; then
        DOWNLOADER="curl"
    elif command -v wget >/dev/null 2>&1; then
        DOWNLOADER="wget"
    else
        print_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    # Check for tar
    if ! command -v tar >/dev/null 2>&1; then
        print_error "tar is required but not found. Please install it."
        exit 1
    fi

    # Check for Rust (optional but recommended)
    if command -v rustc >/dev/null 2>&1; then
        RUST_VERSION=$(rustc --version | cut -d' ' -f2)
        print_success "Rust $RUST_VERSION found"
    else
        print_warning "Rust not found. Consider installing Rust for best Phoenix development experience."
        echo "       Visit https://rustup.rs for installation instructions."
    fi
}

# Download Phoenix CLI
download_phoenix() {
    print_msg "Downloading Phoenix CLI v$PHOENIX_VERSION..." "$BOLD"

    PHOENIX_ARCHIVE="phoenix-$PHOENIX_VERSION-$PLATFORM-$ARCH.tar.gz"
    PHOENIX_URL="$PHOENIX_BASE_URL/v$PHOENIX_VERSION/$PHOENIX_ARCHIVE"

    # Create temporary directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT

    # Download archive
    if [ "$DOWNLOADER" = "curl" ]; then
        if ! curl -fsSL "$PHOENIX_URL" -o "$TMP_DIR/$PHOENIX_ARCHIVE"; then
            # Fallback to building from source
            print_warning "Binary not available. Building from source..."
            build_from_source
            return
        fi
    else
        if ! wget -q "$PHOENIX_URL" -O "$TMP_DIR/$PHOENIX_ARCHIVE"; then
            print_warning "Binary not available. Building from source..."
            build_from_source
            return
        fi
    fi

    # Extract archive
    print_info "Extracting Phoenix CLI..."
    tar -xzf "$TMP_DIR/$PHOENIX_ARCHIVE" -C "$TMP_DIR"

    # Create installation directory
    mkdir -p "$PHOENIX_BIN_DIR"

    # Copy binary
    cp "$TMP_DIR/phoenix" "$PHOENIX_BIN_DIR/phoenix"
    chmod +x "$PHOENIX_BIN_DIR/phoenix"

    print_success "Phoenix CLI downloaded successfully"
}

# Build Phoenix from source
build_from_source() {
    print_msg "Building Phoenix from source..." "$BOLD"

    # Check for Rust
    if ! command -v cargo >/dev/null 2>&1; then
        print_error "Rust is required to build from source. Please install Rust first."
        echo "       Visit https://rustup.rs for installation instructions."
        exit 1
    fi

    # Install using cargo
    print_info "Installing Phoenix CLI via cargo..."
    cargo install phoenix-cli --version "$PHOENIX_VERSION"

    # Find installed binary
    if command -v phoenix >/dev/null 2>&1; then
        print_success "Phoenix CLI installed via cargo"
    else
        print_error "Failed to install Phoenix CLI"
        exit 1
    fi
}

# Setup shell integration
setup_shell() {
    print_msg "Setting up shell integration..." "$BOLD"

    # Detect shell
    SHELL_NAME="$(basename "$SHELL")"

    case "$SHELL_NAME" in
        bash)
            SHELL_CONFIG="$HOME/.bashrc"
            ;;
        zsh)
            SHELL_CONFIG="$HOME/.zshrc"
            ;;
        fish)
            SHELL_CONFIG="$HOME/.config/fish/config.fish"
            ;;
        *)
            print_warning "Unknown shell: $SHELL_NAME. Please add $PHOENIX_BIN_DIR to your PATH manually."
            return
            ;;
    esac

    # Add Phoenix to PATH
    if [ "$SHELL_NAME" = "fish" ]; then
        echo "set -gx PATH \$PATH $PHOENIX_BIN_DIR" >> "$SHELL_CONFIG"
    else
        echo "export PATH=\"\$PATH:$PHOENIX_BIN_DIR\"" >> "$SHELL_CONFIG"
    fi

    # Add completion
    if [ -f "$PHOENIX_INSTALL_DIR/completion/phoenix.bash" ]; then
        case "$SHELL_NAME" in
            bash)
                echo "source $PHOENIX_INSTALL_DIR/completion/phoenix.bash" >> "$SHELL_CONFIG"
                ;;
            zsh)
                echo "source $PHOENIX_INSTALL_DIR/completion/phoenix.zsh" >> "$SHELL_CONFIG"
                ;;
        esac
    fi

    print_success "Shell integration configured for $SHELL_NAME"
}

# Initialize Phoenix configuration
init_config() {
    print_msg "Initializing Phoenix configuration..." "$BOLD"

    CONFIG_DIR="$HOME/.config/phoenix"
    mkdir -p "$CONFIG_DIR"

    # Create default configuration
    cat > "$CONFIG_DIR/config.toml" << EOF
# Phoenix SDK Configuration
version = "$PHOENIX_VERSION"

[defaults]
performance_tier = "Development"
security_level = "Standard"
auto_optimize = true
enable_metrics = true

[development]
hot_reload = true
verbose_logging = false

[telemetry]
enabled = false
anonymous = true
EOF

    print_success "Configuration initialized at $CONFIG_DIR/config.toml"
}

# Run post-installation checks
post_install_check() {
    print_msg "Running post-installation checks..." "$BOLD"

    # Check if Phoenix is accessible
    if "$PHOENIX_BIN_DIR/phoenix" --version >/dev/null 2>&1; then
        VERSION=$("$PHOENIX_BIN_DIR/phoenix" --version | cut -d' ' -f2)
        print_success "Phoenix CLI v$VERSION installed successfully"
    else
        print_error "Phoenix CLI installation verification failed"
        exit 1
    fi

    # Check network connectivity
    if curl -s -o /dev/null -w "%{http_code}" https://api.phoenix.dev/health | grep -q "200"; then
        print_success "Phoenix API is accessible"
    else
        print_warning "Cannot reach Phoenix API. Check your internet connection."
    fi
}

# Interactive setup
interactive_setup() {
    print_msg "Would you like to run the interactive setup? [Y/n] " "$CYAN"
    read -r response

    if [[ ! "$response" =~ ^[Nn]$ ]]; then
        print_info "Starting interactive setup..."
        "$PHOENIX_BIN_DIR/phoenix" tutorial --interactive
    fi
}

# Main installation flow
main() {
    clear
    print_banner

    print_msg "Installing Phoenix SDK..." "$BOLD"
    echo ""

    # Run installation steps
    detect_platform
    check_dependencies
    download_phoenix
    setup_shell
    init_config
    post_install_check

    echo ""
    print_msg "🎉 Phoenix SDK installed successfully!" "$GREEN$BOLD"
    echo ""

    # Print next steps
    print_msg "Next steps:" "$YELLOW$BOLD"
    echo ""
    echo "  1. Reload your shell configuration:"
    echo "     ${CYAN}source ~/.bashrc${RESET}  # or ~/.zshrc"
    echo ""
    echo "  2. Verify installation:"
    echo "     ${CYAN}phoenix --version${RESET}"
    echo ""
    echo "  3. Create your first Phoenix app:"
    echo "     ${CYAN}phoenix new my-app${RESET}"
    echo "     ${CYAN}cd my-app${RESET}"
    echo "     ${CYAN}phoenix dev${RESET}"
    echo ""
    echo "  4. Explore the documentation:"
    echo "     ${CYAN}phoenix docs${RESET}"
    echo ""

    # Optional interactive setup
    interactive_setup

    print_msg "Happy coding with Phoenix! 🚀" "$GREEN$BOLD"
}

# Handle errors
trap 'print_error "Installation failed. Please check the error messages above."' ERR

# Run main installation
main