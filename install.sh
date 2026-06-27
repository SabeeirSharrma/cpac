#!/usr/bin/env bash
set -euo pipefail

# CPAC Installer
# Builds cpac from source and installs it to /usr/local/bin.
# If Rust is not already installed, it installs it temporarily and removes it after.
#
# Usage:
#   curl -sSf https://thecinderproject.qd.je/cpac/install.sh | bash
#
# Environment variables:
#   CPAC_INSTALL_DIR  — install directory (default: /usr/local/bin)
#   CPAC_GIT_REPO     — git repo URL (default: https://github.com/SabeeirSharrma/cpac.git)

REPO="${CPAC_GIT_REPO:-https://github.com/SabeeirSharrma/cpac.git}"
INSTALL_DIR="${CPAC_INSTALL_DIR:-/usr/local/bin}"
BRANCH="${CPAC_BRANCH:-main}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${GREEN}▸${NC} $*"; }
warn()  { echo -e "${YELLOW}▸${NC} $*"; }
error() { echo -e "${RED}▸${NC} $*" >&2; }
step()  { echo -e "\n${CYAN}── $* ──${NC}"; }

# ── Pre-flight: detect what's already installed ──

RUST_WAS_PRESENT=0
GIT_WAS_PRESENT=0

if command -v rustc &>/dev/null && command -v cargo &>/dev/null; then
    RUST_WAS_PRESENT=1
    info "Rust is already installed (rustc $(rustc --version | awk '{print $2}'))"
else
    info "Rust is not installed — will install temporarily for building"
fi

if command -v git &>/dev/null; then
    GIT_WAS_PRESENT=1
    info "Git is already installed"
else
    info "Git is not installed — will install temporarily for cloning"
fi

# ── Helpers ──

cleanup() {
    if [ "${RUST_WAS_PRESENT}" -eq 0 ] && command -v rustup &>/dev/null; then
        step "Cleaning up: removing temporary Rust installation"
        rustup self uninstall -y 2>/dev/null || rm -rf "$HOME/.rustup" "$HOME/.cargo"
        info "Rust removed"
    fi

    if [ "${GIT_WAS_PRESENT}" -eq 0 ] && command -v pacman &>/dev/null; then
        # Only uninstall git if we installed it (check if it was pulled in as a dependency)
        # Skip this — git is commonly needed, don't remove it
        :
    fi

    # Always clean up build directory
    if [ -d "${BUILD_DIR:-}" ]; then
        rm -rf "$BUILD_DIR"
    fi
}

trap cleanup EXIT

install_rust() {
    step "Installing Rust (temporary)"

    if command -v pacman &>/dev/null; then
        sudo pacman -S --noconfirm rustup 2>/dev/null || true
    elif command -v apt-get &>/dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    else
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    fi

    # Source cargo env
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    elif [ -f "$HOME/.cargo/env.fish" ]; then
        # For fish users — just set PATH directly
        export PATH="$HOME/.cargo/bin:$PATH"
    fi

    rustup install stable 2>/dev/null || true
    info "Rust $(rustc --version | awk '{print $2}') installed"
}

install_git() {
    step "Installing Git (temporary)"

    if command -v pacman &>/dev/null; then
        sudo pacman -S --noconfirm git
    elif command -v apt-get &>/dev/null; then
        sudo apt-get update -qq && sudo apt-get install -y -qq git
    else
        error "Cannot install git automatically. Please install git and retry."
        exit 1
    fi

    info "Git installed"
}

# ── Main ──

main() {
    echo ""
    echo -e "  ${CYAN}CPAC Installer${NC}"
    echo -e "  Build from source | github.com/SabeeirSharrma/cpac"
    echo ""

    # Ensure we have git
    if ! command -v git &>/dev/null; then
        install_git
    fi

    # Ensure we have Rust
    if ! command -v rustc &>/dev/null || ! command -v cargo &>/dev/null; then
        install_rust
    fi

    # Create temp build directory
    BUILD_DIR=$(mktemp -d)
    cd "$BUILD_DIR"

    step "Cloning cpac"
    git clone --depth 1 --branch "$BRANCH" "$REPO" cpac
    cd cpac

    step "Building release binary"
    info "This may take a few minutes on first build..."
    cargo build --release 2>&1 | tail -3

    step "Installing cpac"
    local binary="target/release/cpac"

    if [ ! -f "$binary" ]; then
        error "Build failed — binary not found at $binary"
        exit 1
    fi

    if [ -w "$INSTALL_DIR" ]; then
        cp "$binary" "${INSTALL_DIR}/cpac"
    else
        sudo cp "$binary" "${INSTALL_DIR}/cpac"
    fi

    chmod +x "${INSTALL_DIR}/cpac"

    # Verify
    if command -v cpac &>/dev/null; then
        local version
        version=$(cpac --version 2>/dev/null | head -1 | awk '{print $2}' || echo "unknown")
        info "Installed cpac ${version} to ${INSTALL_DIR}/cpac"
    else
        info "Installed cpac to ${INSTALL_DIR}/cpac"
        warn "If 'cpac' is not found, add ${INSTALL_DIR} to your PATH:"
        warn "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi

    echo ""
    info "Run 'cpac --help' to get started"
    echo ""
}

main "$@"
