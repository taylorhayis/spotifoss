default:
    @just --list

build:
    cargo build

build-release:
    cargo build --release

run *ARGS:
    cargo run --bin spotix -- {{ARGS}}

run-release *ARGS:
    cargo run --bin spotix --release -- {{ARGS}}

test *ARGS:
    cargo test {{ARGS}}

check:
    cargo check

clippy *ARGS:
    cargo clippy {{ARGS}} -- -D warnings

fmt:
    cargo fmt

fmt-check:
    cargo fmt -- --check

clean:
    cargo clean

# Build macOS .app bundle (run from spotix-gui; output in target/release/bundle/osx/)
bundle:
    cd spotix-gui && cargo bundle --release

# Rebuild release bundle and install to /Applications/Spotix-Dev.app for Dock use (macOS only)
[macos]
bundle-dev:
    bash scripts/rebundle-macos-dev.sh

[linux]
bundle-dev:
    @echo "bundle-dev is macOS only. On Linux, use: just bundle"

[windows]
bundle-dev:
    @echo "bundle-dev is macOS only."

deps-debian:
    sudo apt-get install libssl-dev libgtk-3-dev libcairo2-dev libasound2-dev

deps-fedora:
    sudo dnf install openssl-devel gtk3-devel cairo-devel alsa-lib-devel
