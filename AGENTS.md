# AGENTS.md

## Purpose
This file is for coding agents working in the Spotifoss repository.
Spotifoss is a fork of [Spotix](https://github.com/skyline69/spotix).
It summarizes the commands, repository layout, and coding conventions that are most useful when making safe changes here.

## Repo-Specific Rule Files
- No `.cursorrules` file exists at the repo root.
- No `.cursor/rules/` directory exists.
- No `.github/copilot-instructions.md` file exists.
- Treat this `AGENTS.md` plus `CLAUDE.md` as the primary local agent guidance.
- If Cursor/Copilot rule files are added later, fold them into your workflow.

## Repository Layout
- Root `Cargo.toml` is a virtual workspace manifest.
- Workspace members are `spotifoss-core` and `spotifoss-gui`.
- `spotifoss-core` contains Spotify session, connection, cache, audio, metadata, and player logic.
- `spotifoss-gui` contains the Druid desktop app and the `spotifoss` binary.
- Intentional patch/vendor crates live beside the workspace: `rspotify-model-patch`, `librespot-playback-patch`, and `vendor/wrapped-vec`.
- Do not edit patch/vendor crates unless the change truly belongs upstream or in the patched dependency itself.
- Patch/vendor crates are not normal workspace members; use `--manifest-path` when building or testing them directly.
- Their `Cargo.toml` files are normalized/generated; inspect `Cargo.toml.orig` first if manifest metadata must change.

## Toolchain and Platform Notes
- Use stable Rust; root docs mention Rust `1.65+`, but patched dependencies may need newer stable toolchains.
- Workspace crates use edition `2024`; some patch/vendor crates still use older editions.
- Linux system dependencies:
  - Debian/Ubuntu: `sudo apt-get install libssl-dev libgtk-3-dev libcairo2-dev libasound2-dev`
  - Fedora/RHEL: `sudo dnf install openssl-devel gtk3-devel cairo-devel alsa-lib-devel`
- Cross-compilation for Linux uses `Cross.toml` and installs GTK/OpenSSL/ALSA dev packages in the image.
- macOS bundling is done from `spotifoss-gui/` with `cargo-bundle`.

## Canonical Build Commands
Run these from the repository root unless noted otherwise.
- Build the whole workspace: `cargo build`
- Build release artifacts: `cargo build --release`
- Build only the core crate: `cargo build -p spotifoss-core`
- Build only the GUI crate: `cargo build -p spotifoss-gui`
- Build the GUI with the alternate audio backend: `cargo build -p spotifoss-gui --no-default-features --features cubeb`
- Run the GUI app: `cargo run -p spotifoss-gui --bin spotifoss`
- Run the GUI app in release mode: `cargo run -p spotifoss-gui --release --bin spotifoss`
- Install the app locally from source: `cargo install --locked --path spotifoss-gui`
- Build the macOS bundle from `spotifoss-gui/`: `cargo bundle --release`

## Lint and Formatting Commands
- Canonical lint gate used in CI: `cargo clippy -- -D warnings`
- Lint one workspace crate: `cargo clippy -p spotifoss-core -- -D warnings`
- Lint the GUI crate only: `cargo clippy -p spotifoss-gui -- -D warnings`
- Format the workspace: `cargo fmt --all`
- Check formatting without writing files: `cargo fmt --all --check`
- There is no separate `cargo lint` wrapper; use `clippy` directly.

## Test Commands
- Run all workspace tests: `cargo test --workspace`
- Run tests for one workspace crate: `cargo test -p spotifoss-core`
- Run tests for the GUI crate only: `cargo test -p spotifoss-gui`
- List discovered tests before filtering: `cargo test -p spotifoss-core -- --list`
- Build tests without running them: `cargo test --workspace --no-run`

### Running a Single Test
Use the exact test name whenever possible.
- Single unit test in a workspace crate: `cargo test -p spotifoss-core test_name -- --exact --nocapture`
- Single test by module-path substring: `cargo test -p spotifoss-core module_name::test_name -- --exact --nocapture`
- Single integration test target: `cargo test -p spotifoss-core --test integration_target test_name -- --exact --nocapture`
- If the filter is ambiguous, first run `cargo test -p spotifoss-core -- --list`.

### Patch/Vendor Crate Tests
Because these crates are outside the workspace, address them explicitly.
- Test all tests in the patched rspotify crate: `cargo test --manifest-path rspotify-model-patch/Cargo.toml`
- Run one exact test in the patched rspotify crate: `cargo test --manifest-path rspotify-model-patch/Cargo.toml test_name -- --exact --nocapture`
- Run one integration test in `wrapped-vec`: `cargo test --manifest-path vendor/wrapped-vec/Cargo.toml --test test test_name -- --exact --nocapture`
- For any non-workspace crate, prefer `--manifest-path <crate>/Cargo.toml` over `-p` from the workspace root.

### Current Testing Reality
- Most concrete tests currently live in patch/vendor crates.
- `spotifoss-core` and `spotifoss-gui` currently rely more on `cargo build` and `cargo clippy` than on broad first-party test coverage.
- When touching workspace code, at minimum run the narrowest relevant build or clippy command even if no direct test target exists.

## Release and Packaging Commands
- Cross-build Linux release binaries: `cross build --release --target x86_64-unknown-linux-gnu`
- Cross-build Linux ARM64 release binaries: `cross build --release --target aarch64-unknown-linux-gnu`
- Add Apple targets when building universal macOS releases: `rustup target add x86_64-apple-darwin aarch64-apple-darwin`
- Build both macOS targets: `cargo build --release --target x86_64-apple-darwin --target aarch64-apple-darwin`
- Build the macOS app bundle from `spotifoss-gui/` after installing `cargo-bundle`.

## Code Style: Formatting and Structure
- Follow `cargo fmt`; do not hand-format against rustfmt output.
- Use the existing import grouping style:
  1. `std` imports
  2. third-party crate imports
  3. `crate::...` imports
  4. `self::...` / `super::...` imports
- Keep blank lines between import groups when multiple groups exist.
- Prefer nested import braces for related items rather than many one-line `use` statements.
- Keep modules and files in `snake_case`.
- Keep type names, traits, and enums in `UpperCamelCase`.
- Keep functions, methods, and local variables in `snake_case`.
- Keep constants and env-var names in `UPPER_SNAKE_CASE`.
- Prefer small helper functions over deeply nested inline logic when a block has a clear name.
- Preserve existing module boundaries instead of stuffing unrelated logic into large files.

## Code Style: Types and Data Modeling
- Derive traits aggressively when they add value: `Clone`, `Debug`, `Default`, `Serialize`, `Deserialize`, `Data`, `Lens`, `Eq`, `PartialEq`.
- In GUI state, prefer `druid::Data`-friendly shapes and shared ownership such as `Arc<T>` when data is shown in multiple places.
- Use enums for small closed sets of options such as `AudioQuality`, `PlaybackEngine`, or `PreferencesTab`.
- Derive `Copy` for lightweight enums and value types when it simplifies call sites and matches existing code.
- Keep config fallback logic in helper methods instead of duplicating it at every call site.
- Prefer explicit state structs over ad-hoc tuples for UI data that grows over time.

## Code Style: Error Handling
- In `spotifoss-core`, prefer `Result<T, spotifoss_core::error::Error>` and propagate recoverable failures with `?`.
- Add or reuse typed `Error` variants when the core layer needs to preserve intent across boundaries.
- In GUI code, it is normal to log an error or warning and degrade gracefully for cache, network, snapshot, or UI update failures.
- Convert errors to strings only at UI-facing boundaries where the user-facing layer truly just needs display text.
- Reserve `expect(...)` for startup invariants, impossible states, or other unrecoverable failures with a specific message.
- Avoid introducing fresh `unwrap()` calls in new code unless the invariant is airtight and already documented by surrounding control flow.
- Prefer `ok_or(...)`, `ok_or_else(...)`, and `map_err(...)` over manual `match` noise when they keep the code readable.

## Code Style: Logging and Diagnostics
- Use the `log` crate macros already used throughout the repo.
- Prefix log messages with the subsystem when helpful, for example `webapi:` or `desktop integration:`.
- Use `warn!` for recoverable failures, `error!` for user-visible breakage or lost functionality, `info!` for lifecycle events, and `debug!` for noisy tracing.
- When ignoring an error intentionally, prefer logging it if the failure could explain user-facing behavior later.

## Code Style: Tests
- Prefer `#[cfg(test)] mod tests` for unit tests next to the code they cover.
- Use `tests/` integration tests only when the behavior crosses modules or needs a public API surface.
- Keep test names descriptive and behavior-focused.
- When adding a new test, document the narrowest command that runs it.

## Practical Guidance for Agents
- Start with the smallest possible change in the correct crate.
- Check whether a change belongs in workspace code or in a patched dependency before editing files outside `spotifoss-core` or `spotifoss-gui`.
- Preserve platform support; avoid Linux-only or macOS-only assumptions unless the code is already gated by `cfg`.
- If you touch audio, session, playback, or cache code, prefer validating with both `cargo build` and a targeted `cargo clippy` run.
- If you touch GUI state types, watch for `druid::Data` and serialization derives so state remains cloneable, comparable, and storable.
- If you touch patch/vendor manifests, verify whether the real change belongs in `Cargo.toml.orig` instead of the normalized `Cargo.toml`.
