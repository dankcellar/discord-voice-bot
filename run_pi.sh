#!/bin/bash
set -e

# Ensure environment variables are loaded for Rust
source $HOME/.cargo/env 2>/dev/null || true

# Set library path for Vosk
export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:$(pwd)/lib

# Build and Run
echo "Starting Discord Voice Bot..."
cargo run --release
