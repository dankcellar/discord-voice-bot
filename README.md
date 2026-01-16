# Discord Voice Bot (Rust Edition)

High-performance Discord voice bot written in Rust, optimized for Raspberry Pi. Features real-time speech transcription using Vosk and an HTTP control interface. Replaces the legacy Node.js/Docker implementation handling UDP voice packets efficiently directly in Rust.

## 🚀 Features

- **Rust Native**: Extremely low resource usage compared to Node.js/Docker.
- **Speech-to-Text**: On-device transcription using Vosk (no cloud API fees).
- **HTTP Control**: Rest API to control the bot externally.
- **CI/CD Ready**: Scripts for automated updates.

## 📦 Requirements

- Raspberry Pi 3B+ / 4 / 5 (ARM64 recommended)
- Raspberry Pi OS (64-bit preferred)
- Internet Connection

## 🛠️ Setup (Raspberry Pi)

1. **Clone the repository**:
   ```bash
   git clone https://github.com/dankcellar/discord-voice-bot.git
   cd discord-voice-bot
   ```

2. **Run the Setup Script**:
   This script installs Rust, system dependencies, downloads the Vosk model/library, and creates a default config.
   ```bash
   chmod +x setup_pi.sh
   ./setup_pi.sh
   ```

3. **Configure**:
   Edit `.env` and add your Discord Bot Token:
   ```bash
   nano .env
   # DISCORD_TOKEN=your_token_here
   ```

4. **Run Manually (Test)**:
   ```bash
   chmod +x run_pi.sh
   ./run_pi.sh
   ```

## 🤖 Hands-off Deployment (Systemd)

To run the bot as a background service that starts on boot:

1. **Copy the service file**:
   ```bash
   sudo cp voice-bot.service /etc/systemd/system/
   ```

2. **Reload Daemon and Start**:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable voice-bot
   sudo systemctl start voice-bot
   ```

3. **Check Status**:
   ```bash
   sudo systemctl status voice-bot
   ```

## 🔄 CI/CD (Continuous Deployment)

To enable automatic updates whenever you push to the `main` branch, you can set up a cron job on the Pi to pull and rebuild periodically.

1. **Test the update script**:
   ```bash
   chmod +x update_bot.sh
   ./update_bot.sh
   ```

2. **Add to Crontab** (Checks for updates every hour):
   ```bash
   crontab -e
   ```
   Add the following line:
   ```bash
   0 * * * * cd /home/pi/discord-voice-bot && ./update_bot.sh >> /home/pi/bot_update.log 2>&1
   ```

## 🎮 Usage

- **Discord Commands**:
  - `!join`: Joins your voice channel.
  - `!leave`: Leaves the voice channel.

- **HTTP Control**:
  - `POST http://pi-ip:3000/control`
    ```json
    { "type": "join", "guildId": 12345, "channelId": 67890 }
    ```

## 📂 Project Structure

- `src/main.rs`: Entry point.
- `src/voice/`: Voice packet handling and Vosk integration.
- `models/`: Stores the Speech-to-Text model.
- `lib/`: Stores the Vosk shared library.
