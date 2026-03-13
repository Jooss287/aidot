#!/bin/bash
# aidot run script - npx-style one-off preset apply
#
# Installs aidot to a temporary directory, runs quick-pull with the given
# arguments, then removes everything. The host system is left unchanged.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Jooss287/aidot/main/scripts/run.sh | bash -s -- <URL> [OPTIONS]
#
# Examples:
#   ... | bash -s -- https://github.com/myteam/llm-config
#   ... | bash -s -- https://github.com/myteam/llm-config --tools claude,cursor
#   ... | bash -s -- ./my-local-preset --dry-run

set -e

REPO="Jooss287/aidot"
BINARY_NAME="aidot"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${GREEN}[aidot-run]${NC} $1"; }
warn()  { echo -e "${YELLOW}[aidot-run]${NC} $1"; }
error() { echo -e "${RED}[aidot-run]${NC} $1"; exit 1; }

# --------------------------------------------------------------------------- #
# Argument validation
# --------------------------------------------------------------------------- #
if [ $# -eq 0 ]; then
    echo -e "${CYAN}Usage:${NC}"
    echo "  curl -fsSL .../run.sh | bash -s -- <PRESET_URL_OR_PATH> [OPTIONS]"
    echo ""
    echo "Options (passed directly to aidot quick-pull):"
    echo "  --tools <list>   Apply to specific tools (comma-separated: claude,cursor,copilot,codex)"
    echo "  --dry-run        Preview changes without applying"
    echo "  --force          Overwrite existing files without asking"
    echo "  --skip           Skip existing files without asking"
    exit 1
fi

PRESET_SOURCE="$1"
shift
QUICK_PULL_ARGS=("$@")

# --------------------------------------------------------------------------- #
# Platform detection (reused from install.sh)
# --------------------------------------------------------------------------- #
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        *)      error "Unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        *)              error "Unsupported architecture: $arch" ;;
    esac

    echo "${arch}-${os}"
}

get_latest_version() {
    local version
    version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
              | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    [ -z "$version" ] && error "Failed to fetch latest version"
    echo "$version"
}

# --------------------------------------------------------------------------- #
# Isolated install to temp directory
# --------------------------------------------------------------------------- #
TEMP_DIR=$(mktemp -d)
# Always clean up on exit (success or failure)
trap 'rm -rf "$TEMP_DIR"' EXIT

info "Creating isolated environment in ${TEMP_DIR}..."

platform=$(detect_platform)
version=$(get_latest_version)
archive_name="aidot-${version}-${platform}.tar.gz"
download_url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"

info "Fetching aidot ${version} (${platform})..."
if ! curl -fsSL "$download_url" -o "${TEMP_DIR}/${archive_name}"; then
    error "Download failed: ${download_url}"
fi

tar -xzf "${TEMP_DIR}/${archive_name}" -C "$TEMP_DIR"
chmod +x "${TEMP_DIR}/${BINARY_NAME}"

AIDOT="${TEMP_DIR}/${BINARY_NAME}"

# --------------------------------------------------------------------------- #
# Run quick-pull
# --------------------------------------------------------------------------- #
info "Running: aidot quick-pull ${PRESET_SOURCE} ${QUICK_PULL_ARGS[*]}"
echo ""

"$AIDOT" quick-pull "$PRESET_SOURCE" "${QUICK_PULL_ARGS[@]}"

echo ""
info "Done. Temporary aidot installation removed."
