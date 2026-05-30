#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_VERSION="$("$SCRIPT_DIR/rust-toolchain.sh")"

log() {
  printf "\033[0;34m>\033[0m %s\n" "$*"
}

die() {
  printf "ERROR: %s\n" "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

add_uv_to_path() {
  export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
}

install_uv() {
  if need_cmd uv; then
    log "uv already installed: $(uv --version)"
    return
  fi

  need_cmd curl || die "curl is required to install uv"
  log "Installing uv..."
  curl -LsSf https://astral.sh/uv/install.sh | sh
  add_uv_to_path
  need_cmd uv || die "uv was installed, but is not on PATH. Add $HOME/.local/bin to PATH and rerun."
  log "Installed $(uv --version)"
}

install_rust() {
  if ! need_cmd rustup; then
    need_cmd curl || die "curl is required to install rustup"
    log "Installing rustup..."
    curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
  fi

  need_cmd rustup || die "rustup is required"

  log "Installing Rust toolchain $RUST_VERSION..."
  rustup toolchain install "$RUST_VERSION"

  # build.py defaults RUSTUP_TOOLCHAIN to "stable" and only accepts stable/nightly.
  # Keep stable current so Cargo satisfies Cargo.toml rust-version.
  log "Updating Rust stable toolchain..."
  rustup update stable
  export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}"
  log "Using $(rustup run "$RUSTUP_TOOLCHAIN" rustc --version)"
}

install_linux_packages() {
  if need_cmd clang && need_cmd make; then
    log "clang and make already installed"
    return
  fi

  log "Installing Linux build packages..."
  if need_cmd apt-get; then
    sudo apt-get update
    sudo apt-get install -y clang make build-essential pkg-config curl
  elif need_cmd dnf; then
    sudo dnf install -y clang make gcc gcc-c++ pkgconf-pkg-config curl
  elif need_cmd yum; then
    sudo yum install -y clang make gcc gcc-c++ pkgconfig curl
  elif need_cmd pacman; then
    sudo pacman -Sy --needed clang make base-devel pkgconf curl
  elif need_cmd zypper; then
    sudo zypper install -y clang make gcc gcc-c++ pkg-config curl
  elif need_cmd apk; then
    sudo apk add clang make build-base pkgconf curl
  else
    die "No supported package manager found. Install clang, make, and a C/C++ build toolchain, then rerun."
  fi
}

check_macos_tools() {
  if xcrun --find clang >/dev/null 2>&1; then
    log "Apple command line tools already installed"
    return
  fi

  log "Apple command line tools are required. Starting installer..."
  xcode-select --install || true
  die "Finish the Apple command line tools install, then rerun this script."
}

install_build_tools() {
  case "$(uname -s)" in
    Linux*) install_linux_packages ;;
    Darwin*) check_macos_tools ;;
    *) die "Unsupported OS for this script. Use scripts/setup-dev.ps1 on Windows." ;;
  esac
}

sync_dependencies() {
  log "Syncing Python dependencies into the project virtual environment..."
  uv sync --all-groups --all-extras --inexact --no-install-package nautilus_trader
}

build_project() {
  log "Building NautilusTrader locally in debug mode..."
  export BUILD_MODE="${BUILD_MODE:-debug}"
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
  export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}"
  uv run --no-sync build.py
}

verify_install() {
  log "Verifying local import..."
  uv run --no-sync python - <<'PY'
import nautilus_trader
from nautilus_trader.core import nautilus_pyo3

print(f"nautilus_trader {nautilus_trader.__version__}")
print(f"pyo3 module {nautilus_pyo3.__name__}")
PY
}

main() {
  cd "$REPO_ROOT"
  add_uv_to_path
  install_build_tools
  install_uv
  install_rust
  sync_dependencies
  build_project
  verify_install
  log "Setup complete. You can now run examples with: uv run --no-sync python examples/backtest/fx_ema_cross_audusd_ticks.py"
}

main "$@"
