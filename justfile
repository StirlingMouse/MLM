# MLM Development Commands
#
# Usage: just <recipe>
# Run `just` or `just --list` to see available commands.

# Variables
dioxus_dir := "mlm_web_dioxus"

# Default: show help
default:
    @just --list

# ============================================================================
# Development (Hot Reloading)
# ============================================================================

# Run Dioxus fullstack dev server with hot patching.
# Loads MLM config and enables server functions during development.
dx-dev:
    cd {{dioxus_dir}} && dx serve --fullstack --web --addr 0.0.0.0

# Alias for dx-dev
dev: dx-dev

# ============================================================================
# Building
# ============================================================================

# Build WASM bundle (debug) - fullstack mode for SSR hydration
dx-build:
    cargo clean -p mlm_web_dioxus
    cd {{dioxus_dir}} && dx build --fullstack

# Build WASM bundle (release)
dx-build-release:
    cd {{dioxus_dir}} && dx build --fullstack --release

# Build the complete server (debug)
build:
    cargo build

# Build the complete server (release)
build-release:
    cargo build --release

# ============================================================================
# Running
# ============================================================================

# Run the main server binary
run:
    cargo run --bin mlm

# Build WASM (debug, skip unchanged static assets) then run server - fastest full app loop
serve:
    cd {{dioxus_dir}} && dx build --fullstack --skip-assets
    cargo run --bin mlm

# Build WASM (debug, full asset copy) then run server
serve-full:
    cd {{dioxus_dir}} && dx build --fullstack
    cargo run --bin mlm

# Build WASM (release) then run server (release)
serve-release: dx-build-release build-release
    cargo run --bin mlm --release

# ============================================================================
# Quality
# ============================================================================

# Run clippy
lint:
    cargo clippy

# Run format check
fmt-check:
    cargo fmt --check

# Run format
fmt:
    cargo fmt

# Run clippy and format check
check: lint fmt-check

# Run tests
test:
    cargo test

# Run e2e Playwright tests (requires a prior `just serve` to have built the WASM)
e2e:
    pnpm exec playwright test

# Build server + test-db fixture binary, then run e2e tests
e2e-build:
    cd {{dioxus_dir}} && dx build --fullstack --skip-assets
    cargo build --bin mlm --bin create_test_db
    pnpm exec playwright test

# ============================================================================
# Cleanup
# ============================================================================

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/dx
