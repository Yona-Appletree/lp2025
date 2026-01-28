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
    cd lp-glsl/apps/esp32-glsl-jit && cargo build --target {{rv32_target}} --release --features esp32c6

# Build RISC-V target packages in release mode
build-rv32-release: install-rv32-target
    @echo "Building RISC-V packages in release mode ({{rv32_target}})..."
    cargo build --target {{rv32_target}} -p lp-builtins-app --release
    cd lp-glsl/apps/esp32-glsl-jit && cargo build --target {{rv32_target}} --release --features esp32c6

# Build all packages (host and RISC-V)
[parallel]
build: build-host build-rv32

# Build all packages in release mode
[parallel]
build-release: build-host-release build-rv32-release


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
# Exclude esp32 packages to avoid esp32 dependency resolution issues
# Note: esp32 packages require feature flags that cause issues during workspace resolution
clippy-host:
    cargo clippy --workspace --exclude lp-builtins-app --exclude esp32-glsl-jit -- --no-deps -D warnings

# Run clippy on RISC-V target packages (esp32-glsl-jit only, lp-builtins-app is mostly generated code)
# Note: esp32-glsl-jit must be built in release mode (esp-hal requirement)
clippy-rv32: install-rv32-target
    @echo "Running clippy on RISC-V packages ({{rv32_target}})..."
    cd lp-glsl/apps/esp32-glsl-jit && cargo clippy --target {{rv32_target}} --release --features esp32c6 -- --no-deps -D warnings

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

# Run all tests
# Note: glsl-filetests.sh handles building builtins automatically
[parallel]
test: test-rust test-filetests

test-rust:
    cargo test

test-filetests:
    scripts/glsl-filetests.sh

# ============================================================================
# CI and validation
# ============================================================================

# Check code for linting, formatting, etc...
[parallel]
check: fmt-check clippy

# Full CI check: build, format check, clippy, and test
[parallel]
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
