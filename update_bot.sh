#!/bin/bash
set -e

# Update Code
echo "Pulling latest changes..."
git pull

# Build
echo "Building release..."
source $HOME/.cargo/env
export LD_LIBRARY_PATH=$(pwd)/lib
cargo build --release

# Restart Service
echo "Restarting service..."
sudo systemctl restart voice-bot

echo "Update Complete!"
