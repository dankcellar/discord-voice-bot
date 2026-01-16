#!/bin/bash
set -e

echo "================================================"
echo "    Discord Voice Bot - Raspberry Pi Setup      "
echo "================================================"

# Check for ARM/Aarch64
ARCH=$(uname -m)
if [[ "$ARCH" != "aarch64" && "$ARCH" != "armv7l" ]]; then
    echo "Warning: This script assumes Raspberry Pi (ARM). Detected: $ARCH"
fi

echo "[1/5] Installing System Dependencies..."
sudo apt-get update
sudo apt-get install -y curl wget unzip build-essential pkg-config libssl-dev cmake libclang-dev libopus-dev

# Install Rust
if ! command -v cargo &> /dev/null; then
    echo "[2/5] Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "[2/5] Rust already installed."
fi

# Download Vosk Model
MODEL_DIR="models"
MODEL_NAME="vosk-model-small-en-us-0.15"
MODEL_URL="https://alphacephei.com/vosk/models/$MODEL_NAME.zip"

if [ ! -d "$MODEL_DIR/$MODEL_NAME" ]; then
    echo "[3/5] Downloading Vosk Model..."
    mkdir -p $MODEL_DIR
    wget -q --show-progress $MODEL_URL -O model.zip
    unzip -q model.zip -d $MODEL_DIR
    rm model.zip
else
    echo "[3/5] Vosk Model found."
fi

# Download Vosk Library
LIB_DIR="lib"
VOSK_VERSION="0.3.45"
if [ "$ARCH" == "aarch64" ]; then
    VOSK_LIB_URL="https://github.com/alphacephei/vosk-api/releases/download/v$VOSK_VERSION/vosk-linux-aarch64-$VOSK_VERSION.zip"
else
    # Fallback for 32-bit armv7l
    VOSK_LIB_URL="https://github.com/alphacephei/vosk-api/releases/download/v$VOSK_VERSION/vosk-linux-armv7l-$VOSK_VERSION.zip"
fi

if [ ! -d "$LIB_DIR" ]; then
    echo "[4/5] Downloading Vosk Shared Library..."
    mkdir -p $LIB_DIR
    wget -q --show-progress $VOSK_LIB_URL -O vosk-lib.zip
    unzip -q vosk-lib.zip -d $LIB_DIR
    
    # Move inner content if needed (zip structure varies, usually has a folder)
    # The zip usually contains 'vosk-linux-aarhc64-x.x' folder
    mv $LIB_DIR/vosk-linux-*/* $LIB_DIR/ 2>/dev/null || true
    rm vosk-lib.zip
else
    echo "[4/5] Vosk Library directory found."
fi

# Setup .env
if [ ! -f .env ]; then
    echo "[5/5] Creating .env file..."
    echo "DISCORD_TOKEN=YOUR_TOKEN_HERE" > .env
    echo "VOSK_MODEL_PATH=$MODEL_DIR/$MODEL_NAME" >> .env
    echo "CONTROL_PORT=3000" >> .env
    echo "Please edit .env with your Discord Token."
fi

echo "================================================"
echo "Setup Complete!"
echo "Run './run_pi.sh' to start the bot."
