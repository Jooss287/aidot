#!/bin/bash
# aidot installer script for Unix/macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/Jooss287/aidot/main/scripts/install.sh | bash

set -e

# Configuration
REPO="Jooss287/aidot"
INSTALL_DIR="${AIDOT_INSTALL_DIR:-$HOME/.aidot/bin}"
BINARY_NAME="aidot"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Markers for shell profile
MARKER_START="# >>> aidot >>>"
MARKER_END="# <<< aidot <<<"

# Detect shell profile file
detect_shell_profile() {
    local shell_name
    shell_name=$(basename "$SHELL")

    case "$shell_name" in
        zsh)
            echo "$HOME/.zshrc"
            ;;
        bash)
            # Prefer .bashrc, fallback to .bash_profile
            if [ -f "$HOME/.bashrc" ]; then
                echo "$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                echo "$HOME/.bash_profile"
            else
                echo "$HOME/.bashrc"
            fi
            ;;
        fish)
            echo "$HOME/.config/fish/config.fish"
            ;;
        *)
            # Default to .profile for other shells
            echo "$HOME/.profile"
            ;;
    esac
}

# Add PATH to shell profile
add_to_path() {
    local profile_file
    profile_file=$(detect_shell_profile)

    # Check if already added by looking for our marker
    if [ -f "$profile_file" ] && grep -q "$MARKER_START" "$profile_file"; then
        info "PATH already configured in $profile_file"
        return 0
    fi

    # Create profile file if it doesn't exist
    if [ ! -f "$profile_file" ]; then
        touch "$profile_file"
    fi

    # Create fish config directory if needed
    if [[ "$profile_file" == *"fish"* ]]; then
        mkdir -p "$(dirname "$profile_file")"
        # Fish shell uses different syntax
        {
            echo ""
            echo "$MARKER_START"
            echo "set -gx PATH \$PATH $INSTALL_DIR"
            echo "$MARKER_END"
        } >> "$profile_file"
    else
        # Bash/Zsh/POSIX shell syntax
        {
            echo ""
            echo "$MARKER_START"
            echo "export PATH=\"\$PATH:$INSTALL_DIR\""
            echo "$MARKER_END"
        } >> "$profile_file"
    fi

    info "Added aidot to PATH in $profile_file"
    echo ""
    echo "To use aidot now, run:"
    echo ""
    echo "  source $profile_file"
    echo ""
    echo "Or open a new terminal."
}

# Remove PATH from shell profile
remove_from_path() {
    local profile_file
    profile_file=$(detect_shell_profile)

    if [ -f "$profile_file" ] && grep -q "$MARKER_START" "$profile_file"; then
        # Remove the aidot block from profile
        sed -i.bak "/$MARKER_START/,/$MARKER_END/d" "$profile_file"
        rm -f "${profile_file}.bak"
        info "Removed aidot from PATH in $profile_file"
    fi
}

# Check if git is installed
check_git() {
    if command -v git &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# Detect OS and architecture
detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            os="unknown-linux-gnu"
            ;;
        Darwin)
            os="apple-darwin"
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac

    case "$arch" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            ;;
    esac

    echo "${arch}-${os}"
}

# Get latest release version
get_latest_version() {
    local version
    local prerelease="${1:-false}"

    if [ "$prerelease" = "true" ]; then
        version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
    else
        version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    fi

    if [ -z "$version" ]; then
        error "Failed to get latest version"
    fi

    echo "$version"
}

# Download and install
install() {
    local platform version download_url temp_dir archive_name
    local prerelease="${1:-false}"
    local no_add_path="${2:-false}"

    # Check if git is installed (required for remote repository features)
    if ! check_git; then
        warn "Git is not installed. Some features (repo add, pull from remote) will not work."
        echo ""
        echo "To install git:"
        echo "  - macOS: brew install git"
        echo "  - Ubuntu/Debian: sudo apt install git"
        echo "  - Fedora: sudo dnf install git"
        echo ""
    fi

    platform=$(detect_platform)
    version=$(get_latest_version "$prerelease")

    info "Installing aidot ${version} for ${platform}..."

    archive_name="aidot-${version}-${platform}.tar.gz"
    download_url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"

    # Create temp directory
    temp_dir=$(mktemp -d)
    trap "rm -rf $temp_dir" EXIT

    # Download
    info "Downloading ${download_url}..."
    if ! curl -fsSL "$download_url" -o "${temp_dir}/${archive_name}"; then
        error "Failed to download aidot. Please check if the release exists for your platform."
    fi

    # Extract
    info "Extracting..."
    tar -xzf "${temp_dir}/${archive_name}" -C "$temp_dir"

    # Install
    info "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    mv "${temp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    # Verify installation
    if [ -x "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        info "Successfully installed aidot to ${INSTALL_DIR}/${BINARY_NAME}"
    else
        error "Installation failed"
    fi

    # Add to PATH if not already there
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        if [ "$no_add_path" = "true" ]; then
            warn "Note: ${INSTALL_DIR} is not in your PATH"
            echo ""
            echo "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo ""
            echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
            echo ""
        else
            add_to_path
        fi
    else
        info "PATH already contains ${INSTALL_DIR}"
    fi

    # Print version
    echo ""
    info "Installation complete!"
    "${INSTALL_DIR}/${BINARY_NAME}" --version || true
}

# Uninstall
uninstall() {
    if [ -f "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        rm -f "${INSTALL_DIR}/${BINARY_NAME}"
        info "Uninstalled aidot from ${INSTALL_DIR}"
        # Remove PATH entry from shell profile
        remove_from_path
    else
        warn "aidot is not installed in ${INSTALL_DIR}"
    fi
}

# Main
COMMAND="${1:-install}"
PRERELEASE="false"
NO_ADD_PATH="false"

# Check for flags
for arg in "$@"; do
    if [ "$arg" = "--prerelease" ] || [ "$arg" = "-prerelease" ]; then
        PRERELEASE="true"
    fi
    if [ "$arg" = "--no-add-path" ] || [ "$arg" = "-no-add-path" ]; then
        NO_ADD_PATH="true"
    fi
done

case "$COMMAND" in
    install)
        install "$PRERELEASE" "$NO_ADD_PATH"
        ;;
    uninstall)
        uninstall
        ;;
    *)
        echo "Usage: $0 [install|uninstall] [--prerelease] [--no-add-path]"
        exit 1
        ;;
esac
