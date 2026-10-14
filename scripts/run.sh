#!/usr/bin/env sh
# Build and run liteweight-fps from source (macOS / Linux).
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

if ! command -v cargo >/dev/null 2>&1; then
    cat >&2 <<'EOF'
Rust toolchain not found. Install Rust stable, then re-run this script:
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  . "$HOME/.cargo/env"
EOF
    exit 1
fi

if [ "$(uname -s)" = "Linux" ]; then
    for tool in pkg-config; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            echo "Note: native build dependencies may be missing. On Debian/Ubuntu:" >&2
            echo "  sudo apt install build-essential pkg-config libasound2-dev libwayland-dev libxkbcommon-dev" >&2
            break
        fi
    done
fi

exec cargo run --release --locked -- "$@"
