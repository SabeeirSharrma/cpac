#!/usr/bin/env bash
set -euo pipefail

# CPAC Installer
# Downloads and installs the latest cpac binary.
# Usage: curl -sSf https://thecinderproject.qd.je/cpac/install.sh | bash

REPO="SabeeirSharrma/cpac"
INSTALL_DIR="${CPAC_INSTALL_DIR:-/usr/local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}▸${NC} $*"; }
warn()  { echo -e "${YELLOW}▸${NC} $*"; }
error() { echo -e "${RED}▸${NC} $*" >&2; }

# Detect architecture
detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)
            error "Unsupported architecture: $arch"
            error "cpac supports x86_64 and aarch64."
            exit 1
            ;;
    esac
}

# Get latest release tag from GitHub
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local version

    if command -v curl &>/dev/null; then
        version=$(curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
    elif command -v wget &>/dev/null; then
        version=$(wget -qO- "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one."
        exit 1
    fi

    if [ -z "$version" ]; then
        error "Failed to fetch latest version from GitHub."
        exit 1
    fi

    echo "$version"
}

# Download file with curl or wget
download() {
    local url="$1"
    local dest="$2"

    if command -v curl &>/dev/null; then
        curl -fsSL -o "$dest" "$url"
    elif command -v wget &>/dev/null; then
        wget -qO "$dest" "$url"
    else
        error "Neither curl nor wget found."
        exit 1
    fi
}

main() {
    echo ""
    echo "  CPAC Installer"
    echo "  Community Package Analysis Client"
    echo ""

    # Check if already installed
    if command -v cpac &>/dev/null; then
        local current_version
        current_version=$(cpac --version 2>/dev/null | head -1 | awk '{print $2}' || true)
        warn "cpac is already installed (version: ${current_version:-unknown})"
    fi

    # Detect
    local arch
    arch=$(detect_arch)
    info "Detected architecture: ${arch}"

    # Get latest version
    info "Fetching latest release..."
    local version
    version=$(get_latest_version)
    info "Latest version: ${version}"

    # Build download URL
    local binary_name="cpac-${arch}-linux"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${binary_name}"
    local checksum_url="https://github.com/${REPO}/releases/download/${version}/sha256sums.txt"

    # Download binary
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    info "Downloading ${binary_name}..."
    download "$download_url" "${tmp_dir}/cpac"

    # Download and verify checksum
    info "Verifying checksum..."
    download "$checksum_url" "${tmp_dir}/sha256sums.txt"

    local expected_hash
    expected_hash=$(grep "${binary_name}" "${tmp_dir}/sha256sums.txt" | awk '{print $1}')

    if [ -z "$expected_hash" ]; then
        warn "Could not find checksum for ${binary_name}, skipping verification"
    else
        local actual_hash
        actual_hash=$(sha256sum "${tmp_dir}/cpac" | awk '{print $1}')

        if [ "$expected_hash" != "$actual_hash" ]; then
            error "Checksum mismatch!"
            error "  Expected: ${expected_hash}"
            error "  Got:      ${actual_hash}"
            exit 1
        fi
        info "Checksum verified"
    fi

    # Install
    chmod +x "${tmp_dir}/cpac"

    if [ -w "$INSTALL_DIR" ]; then
        cp "${tmp_dir}/cpac" "${INSTALL_DIR}/cpac"
    else
        info "Installing to ${INSTALL_DIR} (requires sudo)..."
        sudo cp "${tmp_dir}/cpac" "${INSTALL_DIR}/cpac"
    fi

    info "Installed cpac ${version} to ${INSTALL_DIR}/cpac"
    echo ""
    info "Run 'cpac --help' to get started"
    echo ""
}

main "$@"
