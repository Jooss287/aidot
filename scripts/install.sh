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

    # Check if INSTALL_DIR is in PATH
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        warn "Note: ${INSTALL_DIR} is not in your PATH"
        echo ""
        echo "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
        echo ""
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
    else
        warn "aidot is not installed in ${INSTALL_DIR}"
    fi
}

# Main
COMMAND="${1:-install}"
PRERELEASE="false"

# Check for --prerelease flag
for arg in "$@"; do
    if [ "$arg" = "--prerelease" ] || [ "$arg" = "-prerelease" ]; then
        PRERELEASE="true"
    fi
done

case "$COMMAND" in
    install)
        install "$PRERELEASE"
        ;;
    uninstall)
        uninstall
        ;;
    *)
        echo "Usage: $0 [install|uninstall] [--prerelease]"
        exit 1
        ;;
esac
