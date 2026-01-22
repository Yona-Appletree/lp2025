# LightPlayer justfile
# Common development tasks

# Variables
rv32_target := "riscv32imac-unknown-none-elf"
rv32_packages := "esp32-glsl-jit lp-builtins-app"
lp_glsl_dir := "lp-glsl"

# Default recipe - show available commands
default:
    @just --list

# ============================================================================
# Target setup
# ============================================================================

# Ensure RISC-V target is installed
install-rv32-target:
    @if ! rustup target list --installed | grep -q "^{{rv32_target}}$"; then \
        echo "Installing target {{rv32_target}}..."; \
        rustup target add {{rv32_target}}; \
    else \
        echo "Target {{rv32_target}} already installed"; \
    fi

# Generate builtin boilerplate code
generate-builtins:
    cargo run --bin lp-builtin-gen -p lp-builtin-gen

# ============================================================================
# Build commands
# ============================================================================

# Build host target packages (default workspace)
build-host:
    cargo build

# Build host target packages in release mode
build-host-release:
    cargo build --release

# Build RISC-V target packages
# Note: esp32-glsl-jit is too large for debug builds, so it's built in release mode
build-rv32: install-rv32-target
    @echo "Building RISC-V packages ({{rv32_target}})..."
    cargo build --target {{rv32_target}} -p lp-builtins-app
    cargo build --target {{rv32_target}} -p esp32-glsl-jit --release

# Build RISC-V target packages in release mode
build-rv32-release: install-rv32-target
    @echo "Building RISC-V packages in release mode ({{rv32_target}})..."
    cargo build --target {{rv32_target}} -p esp32-glsl-jit -p lp-builtins-app --release

# Build all packages (host and RISC-V)
build: build-host build-rv32

# Build all packages in release mode
build-release: build-host-release build-rv32-release

# Build lp-builtins-app with special flags for filetests
# This is required before running tests that use the emulator
filetests-setup: generate-builtins install-rv32-target
    @echo "Building lp-builtins-app for filetests ({{rv32_target}})..."
    RUSTFLAGS="-C opt-level=1 -C panic=abort -C overflow-checks=off -C debuginfo=0 -C link-dead-code=off -C codegen-units=1" \
    cargo build \
        --target {{rv32_target}} \
        --package lp-builtins-app \
        --release
    @if command -v nm > /dev/null 2>&1; then \
        LP_SYMBOLS=`nm target/{{rv32_target}}/release/lp-builtins-app 2>/dev/null | grep "__lp_" | wc -l | xargs`; \
        echo "✓ lp-builtins-app built with $LP_SYMBOLS built-ins"; \
    fi

# ============================================================================
# Formatting
# ============================================================================

# Format code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# ============================================================================
# Linting
# ============================================================================

# Run clippy on host target packages
clippy-host:
    cargo clippy --workspace --exclude lp-builtins-app --exclude esp32-glsl-jit -- --no-deps -D warnings

# Run clippy on RISC-V target packages (esp32-glsl-jit only, lp-builtins-app is mostly generated code)
# Note: esp32-glsl-jit must be built in release mode (esp-hal requirement)
clippy-rv32: install-rv32-target
    @echo "Running clippy on RISC-V packages ({{rv32_target}})..."
    cargo clippy --target {{rv32_target}} -p esp32-glsl-jit --release -- --no-deps -D warnings

# Run clippy on all packages (host and RISC-V)
clippy: clippy-host clippy-rv32

# Run clippy and auto-fix issues
clippy-fix:
    cargo clippy --fix --allow-dirty --allow-staged

# Format code and auto-fix clippy issues
fix: fmt clippy-fix

# ============================================================================
# Testing
# ============================================================================

# Run all tests (requires filetests-setup)
test: filetests-setup
    cargo test

# Run GLSL filetests specifically
test-filetests: filetests-setup
    cargo test -p lp-glsl-filetests --test filetests

# ============================================================================
# CI and validation
# ============================================================================

# Check code for linting, formatting, etc...
check: fmt-check clippy

# Full CI check: build, format check, clippy, and test
ci: check build test

# ============================================================================
# Cleanup
# ============================================================================

# Clean build artifacts
clean:
    cargo clean

# Clean everything including target directories
clean-all: clean
    rm -rf {{lp_glsl_dir}}/target

# ============================================================================
# Git workflows
# ============================================================================

# Push changes to origin and create/update PR
push: check
    scripts/push.sh

# Push changes, run ci, and merge PR if successful
merge: check
    scripts/push.sh --merge
