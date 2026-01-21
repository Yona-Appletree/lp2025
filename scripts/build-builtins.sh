#!/bin/bash
# Build lp-builtins-app executable with aggressive optimizations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIGHTPLAYER_DIR="$WORKSPACE_ROOT/lp-glsl"
BUILTINS_APP="$LIGHTPLAYER_DIR/apps/lp-builtins-app"
TARGET="riscv32imac-unknown-none-elf"
OUTPUT_DIR="$LIGHTPLAYER_DIR/target/$TARGET/release"

echo "Building lp-builtins-app for $TARGET with aggressive optimizations..."

# Generate builtin boilerplate code
echo "Generating builtin boilerplate..."
cd "$LIGHTPLAYER_DIR"
cargo run --bin lp-builtin-gen --manifest-path apps/lp-builtin-gen/Cargo.toml

# Ensure target is installed
if ! rustup target list --installed | grep -q "^$TARGET$"; then
    echo "Installing target $TARGET..."
    rustup target add $TARGET
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build using cargo but with RUSTFLAGS for optimization
# Using rustc directly would require handling dependencies manually
cd "$LIGHTPLAYER_DIR"

RUSTFLAGS="-C opt-level=1 \
           -C panic=abort \
           -C overflow-checks=off \
           -C debuginfo=0 \
           -C link-dead-code=off \
           -C codegen-units=1" \
cargo build \
    --target $TARGET \
    --package lp-builtins-app \
    --release \
    --bin lp-builtins-app

# Count symbols
LP_SYMBOLS=$(nm "$OUTPUT_DIR/lp-builtins-app" 2>/dev/null | grep "__lp_" | wc -l | xargs)

# Output formatted results
GREEN='\033[0;32m'
NC='\033[0m' # No Color
echo -e "${GREEN}lp-builtins-app:${NC} built with $LP_SYMBOLS built-ins"

