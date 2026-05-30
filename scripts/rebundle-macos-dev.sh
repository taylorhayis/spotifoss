#!/usr/bin/env bash
# Rebuild Spotix release bundle and install to /Applications/Spotix-Dev.app
# so a Dock shortcut always launches the latest local build.
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "rebundle-macos-dev.sh is macOS only." >&2
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if ! command -v cargo-bundle >/dev/null 2>&1; then
    echo "cargo-bundle not found. Install with: cargo install cargo-bundle" >&2
    exit 1
fi

cd "$ROOT/spotix-gui"
echo "Building release bundle..."
cargo bundle --release

APP_SRC="$ROOT/target/release/bundle/osx/Spotix.app"
APP_DST="/Applications/Spotix-Dev.app"

rm -rf "$APP_DST"
cp -R "$APP_SRC" "$APP_DST"

echo
echo "Installed $APP_DST"
echo "Launch from Dock, or run: open $APP_DST"
echo
echo "Re-run after code changes:"
echo "  just bundle-dev"
echo "  bash scripts/rebundle-macos-dev.sh"
