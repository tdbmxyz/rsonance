# Audio Transmission Tool

## Project Overview
A Rust tool that captures microphone audio and transmits it to another device for playback through a virtual audio input device. This enables remote desktop software to capture audio from a remote microphone.

## Architecture
- **Transmitter**: Captures microphone audio using `cpal` and streams via TCP
- **Receiver**: Creates virtual microphone input and pipes received audio
- **Core**: Real-time audio streaming with minimal latency and reconnection support

## Key Dependencies
- `cpal` - Cross-platform audio I/O
- `tokio` - Async runtime for networking
- `anyhow` - Error handling

## Usage

### 1. Start the Receiver (Virtual Microphone)
On the remote desktop machine:
```bash
cargo run --bin receiver [OPTIONS]
```

**Common usage examples:**
```bash
# Start with default settings (localhost:8080)
cargo run --bin receiver

# Bind to all interfaces on custom port
cargo run --bin receiver --host 0.0.0.0 --port 9090

# Custom microphone name and verbose output
cargo run --bin receiver --microphone-name my_virtual_mic --verbose

# Custom FIFO path and buffer size
cargo run --bin receiver --fifo-path /tmp/my_audio_pipe --buffer-size 8192
```

This creates a virtual microphone device that remote desktop software can use.

### 2. Start the Transmitter (Microphone Capture)
On the local machine with the microphone:
```bash
cargo run --bin transmitter [OPTIONS]
```

**Common usage examples:**
```bash
# Connect to default localhost:8080
cargo run --bin transmitter

# Connect to remote server
cargo run --bin transmitter --host 192.168.1.100 --port 9090

# Enable verbose output and custom buffer size
cargo run --bin transmitter --verbose --buffer-size 8192

# Set maximum reconnection attempts
cargo run --bin transmitter --reconnect-attempts 10
```

### 3. Configure Remote Desktop Software
In your remote desktop application, select "Mike_Virtual_Microphone" as the microphone input device.

## Features
- **Virtual Audio Input**: Creates a virtual microphone that appears as a real input device
- **Low Latency**: Optimized for real-time audio transmission
- **Auto-Reconnection**: Transmitter automatically reconnects if connection drops
- **Multiple Formats**: Supports F32, I16, and U16 audio sample formats
- **PulseAudio Compatible**: Works with PulseAudio and PipeWire (via PA compatibility)
- **Auto-Cleanup**: Automatically removes virtual device on exit (Ctrl+C)

## Commands
- Build: `cargo build --release`
- Run receiver: `cargo run --bin receiver [OPTIONS]`
- Run transmitter: `cargo run --bin transmitter [OPTIONS]`
- Test: `cargo test`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Build all targets: `cargo build --all-targets`
- Add dependencies: `cargo add <crate_name>` (preferred over editing Cargo.toml directly)
- Help: `cargo run --bin <receiver|transmitter> -- --help`

## CLI Options

### Receiver Options
- `--host/-H <HOST>`: Host address to bind to (default: 0.0.0.0)
- `--port/-p <PORT>`: Port to listen on (default: 8080)
- `--buffer-size/-b <SIZE>`: Audio buffer size in bytes (default: 4096)
- `--microphone-name/-m <NAME>`: Virtual microphone name (default: mike_virtual_microphone)
- `--fifo-path/-f <PATH>`: FIFO pipe path (default: /tmp/mike_audio_pipe)
- `--verbose/-v`: Enable verbose output

### Transmitter Options
- `--host/-H <HOST>`: Server address to connect to (default: 127.0.0.1)
- `--port/-p <PORT>`: Server port to connect to (default: 8080)
- `--buffer-size/-b <SIZE>`: Audio buffer size in bytes (default: 4096)
- `--reconnect-attempts/-r <NUM>`: Max reconnection attempts (default: 5)
- `--verbose/-v`: Enable verbose output

## Development
- **Testing**: Comprehensive unit tests for all core functions
- **Linting**: Clippy-clean codebase with warnings as errors
- **Modular Design**: Core logic extracted to library for testability
- **Error Handling**: Proper error propagation with anyhow

## System Requirements
- Linux with PulseAudio or PipeWire (with PulseAudio compatibility)
- `pactl` command (usually included with PulseAudio/PipeWire)
- Network connectivity between transmitter and receiver
- Microphone on transmitter machine

## Cleanup
- The receiver automatically cleans up the virtual microphone when you press Ctrl+C
- Virtual device is completely removed from the system
- FIFO pipe is also cleaned up automatically

## Troubleshooting
- Check if PulseAudio/PipeWire is running: `pactl info`
- List audio sources: `pactl list sources short`
- Verify virtual microphone: Look for "mike_virtual_microphone" in sources
- Ensure firewall allows TCP connections on port 8080
- Manual cleanup if needed: `pactl unload-module <module_id>` (find ID with `pactl list modules short | grep pipe-source`)
- On Hyprland/Wayland: Ensure audio session is properly configured