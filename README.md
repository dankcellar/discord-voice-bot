# Discord Voice Bot

A Rust implementation of a Discord bot that transcribes audio from voice channels using the Vosk speech recognition model and forwards transcriptions to an external API.

## Features

- Real-time audio transcription using Vosk
- Discord voice channel integration via Songbird
- Automatic forwarding of transcriptions to external API
- Asynchronous processing with Tokio
- Structured logging with tracing

## Prerequisites

- Rust 1.70 or later
- Discord bot token
- Vosk language model (download from https://alphacephei.com/vosk/models)
- External API endpoint for receiving transcriptions

## Installation

1. Clone the repository:
```bash
git clone https://github.com/dankcellar/discord-voice-bot.git
cd discord-voice-bot
```

2. Download a Vosk model:
```bash
# Example: English model
wget https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip
unzip vosk-model-en-us-0.22.zip
```

3. Create a `.env` file from the example:
```bash
cp .env.example .env
```

4. Edit `.env` with your configuration:
```env
DISCORD_TOKEN=your_discord_bot_token
VOSK_MODEL_PATH=/path/to/vosk-model-en-us-0.22
API_ENDPOINT=https://your-api.com/transcriptions
```

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

## Configuration

The bot reads configuration from environment variables:

- `DISCORD_TOKEN`: Your Discord bot token
- `VOSK_MODEL_PATH`: Path to the Vosk model directory
- `API_ENDPOINT`: URL of the external API to receive transcriptions

## API Payload Format

Transcriptions are sent to the external API as JSON:

```json
{
  "text": "transcribed text here",
  "user_id": "123456789",
  "timestamp": "2026-01-17T10:30:00Z"
}
```

## Discord Bot Setup

1. Create a bot at https://discord.com/developers/applications
2. Enable the following privileged intents:
   - Guild Voice States
   - Guild Messages
3. Invite the bot with the `bot` and `voice` scopes
4. Required permissions: Connect, Speak, Use Voice Activity

## Architecture

- **serenity**: Discord API client
- **songbird**: Voice channel support and audio handling
- **vosk**: Speech recognition engine
- **reqwest**: HTTP client for API calls
- **tokio**: Async runtime

## Performance

The bot is optimized for production use with:
- LTO (Link Time Optimization)
- Maximum optimization level
- Single codegen unit for better performance

## License

MIT

## Contributing

Pull requests are welcome. For major changes, please open an issue first.
