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
cargo run --bin receiver
```
This creates a virtual microphone device called "Mike_Virtual_Microphone" that remote desktop software can use.

### 2. Start the Transmitter (Microphone Capture)
On the local machine with the microphone:
```bash
cargo run --bin transmitter [SERVER_IP:PORT]
```
Default connects to `127.0.0.1:8080`. Specify a different address for remote connections.

### 3. Configure Remote Desktop Software
In your remote desktop application, select "Mike_Virtual_Microphone" as the microphone input device.

## Features
- **Virtual Audio Input**: Creates a virtual microphone that appears as a real input device
- **Low Latency**: Optimized for real-time audio transmission
- **Auto-Reconnection**: Transmitter automatically reconnects if connection drops
- **Multiple Formats**: Supports F32, I16, and U16 audio sample formats
- **PipeWire Native**: Primary support for PipeWire with PulseAudio fallback

## Commands
- Build: `cargo build --release`
- Run receiver: `cargo run --bin receiver`
- Run transmitter: `cargo run --bin transmitter [IP:PORT]`
- Test: `cargo test`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Build all targets: `cargo build --all-targets`
- Add dependencies: `cargo add <crate_name>` (preferred over editing Cargo.toml directly)

## Development
- **Testing**: Comprehensive unit tests for all core functions
- **Linting**: Clippy-clean codebase with warnings as errors
- **Modular Design**: Core logic extracted to library for testability
- **Error Handling**: Proper error propagation with anyhow

## System Requirements
- Linux with PipeWire (recommended) or PulseAudio
- PipeWire tools: `pw-cli`, `pw-cat` (usually included with PipeWire)
- Network connectivity between transmitter and receiver
- Microphone on transmitter machine

## Troubleshooting
- Check if PipeWire is running: `systemctl --user status pipewire`
- List audio sources: `pw-cli list-objects | grep Audio/Source`
- For PulseAudio fallback, check: `pactl list sources`
- Verify virtual microphone: Look for "Mike Virtual Microphone" in audio settings
- Ensure firewall allows TCP connections on port 8080
- On Hyprland/Wayland: Make sure PipeWire session is properly configured