#!/usr/bin/env bash
# Aura Hot Reload Watcher for macOS / Linux
set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "================================================="
echo "   AURA REST CLIENT - HOT RELOAD WATCHER         "
echo "================================================="

if command -v cargo-watch >/dev/null 2>&1 || cargo --list | grep -q "watch"; then
    echo "[Aura Hot Reload] Running cargo-watch..."
    exec cargo watch -c -w src -w Cargo.toml -x "run --bin aura"
fi

echo "[Aura Hot Reload] 'cargo-watch' is not installed."
echo "[Tip] Run 'cargo install cargo-watch' for optimal hot-reloading."
echo "[Aura Hot Reload] Running Aura in standard mode..."
exec cargo run --bin aura
