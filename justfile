# LightPlayer justfile
# Common development tasks

# Variables
riscv_target := "riscv32imac-unknown-none-elf"
lp_glsl_dir := "lp-glsl"
esp32_app_dir := "lp-glsl/apps/esp32-glsl-jit"
builtins_app_dir := "lp-glsl/apps/lp-builtins-app"
builtins_exe := "target/{{riscv_target}}/release/lp-builtins-app"

# Default recipe - show available commands
default:
    @just --list

# Build workspace (host target only, excludes ESP32 app)
build:
    cargo build

# Build workspace in release mode
build-release:
    cargo build --release

# Generate builtin boilerplate code
generate-builtins:
    cargo run --bin lp-builtin-gen -p lp-builtin-gen

# Ensure RISC-V target is installed
install-target:
    @if ! rustup target list --installed | grep -q "^{{riscv_target}}$"; then \
        echo "Installing target {{riscv_target}}..."; \
        rustup target add {{riscv_target}}; \
    else \
        echo "Target {{riscv_target}} already installed"; \
    fi

# Build lp-builtins-app for emulator tests
# This is required before running tests that use the emulator
# Note: lp-builtins-app is excluded from default-members but still in workspace
build-builtins: generate-builtins install-target
    @echo "Building lp-builtins-app for {{riscv_target}}..."
    RUSTFLAGS="-C opt-level=1 -C panic=abort -C overflow-checks=off -C debuginfo=0 -C link-dead-code=off -C codegen-units=1" \
    cargo build \
        --target {{riscv_target}} \
        --package lp-builtins-app \
        --release
    @if command -v nm > /dev/null 2>&1; then \
        LP_SYMBOLS=`nm target/{{riscv_target}}/release/lp-builtins-app 2>/dev/null | grep "__lp_" | wc -l | xargs`; \
        echo "✓ lp-builtins-app built with $LP_SYMBOLS built-ins"; \
    fi

# Run all tests (builds builtins-app first if needed)
test: build-builtins
    cargo test

# Run GLSL filetests specifically
test-filetests: build-builtins
    cargo test -p lp-glsl-filetests --test filetests

# Format code
fmt:
    cargo fmt --all

# Run clippy and auto-fix issues
clippy-fix:
    cargo clippy --fix --allow-dirty --allow-staged

# Format code and auto-fix clippy issues
fix: fmt clippy-fix

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run clippy checks (exclude lp-builtins-app and esp32-glsl-jit as they're no_std, and exclude dependencies)
clippy:
    cargo clippy --workspace --exclude lp-builtins-app --exclude esp32-glsl-jit -- --no-deps -D warnings

# Run clippy on ESP32 app (requires RISC-V target)
clippy-esp32: install-target
    @echo "Running clippy on ESP32 app (riscv32imac-unknown-none-elf)..."
    cargo clippy --target {{riscv_target}} -p esp32-glsl-jit -- --no-deps -D warnings

# Run clippy on all packages including embedded targets
clippy-all: clippy clippy-esp32

# Build ESP32 app (excluded from default-members, only builds for RISC-V)
esp32-build:
    @echo "Building ESP32 app (riscv32imac-unknown-none-elf)..."
    cargo build --target {{riscv_target}} -p esp32-glsl-jit --release

# Full CI check: build, format check, clippy, and test
ci: fmt-check clippy test

# Clean build artifacts
clean:
    cargo clean

# Clean everything including target directories
clean-all: clean
    rm -rf {{lp_glsl_dir}}/target

# Check code for linting, formatting, etc...
check: fmt-check clippy

# Validate code by checking and testing
validate: check test

# Push changes to origin and create/update PR
push: check
    scripts/push.sh

# Push changes, run ci, and merge PR if successful
merge: check
    scripts/push.sh --merge