# Discord Voice Bot

A Discord bot that transcribes voice channel audio using Vosk speech recognition.

## Build Process

The build script (`build.rs`) automatically handles dependencies during compilation:

1. **Vosk Library Setup** - Downloads platform-specific Vosk native libraries (Windows/Linux/macOS) and copies them to the build directory
2. **Speech Model Download** - Downloads the Vosk speech recognition model (~40MB) to `models/` directory
3. **Validation** - Verifies model integrity before proceeding

On first build, this process takes a few minutes. Subsequent builds reuse existing files.

### Manual Setup

If automatic download fails, set `VOSK_LIB_DIR` environment variable to specify library location manually.

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```
