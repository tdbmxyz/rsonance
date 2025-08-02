# Audio Transmission Tool

## Project Overview
A Rust tool that captures microphone audio and transmits it to another device for playback through a specified or virtual audio input.

## Architecture
- **Input**: Microphone capture using `cpal`
- **Output**: Audio transmission to remote device
- **Core**: Real-time audio streaming with minimal latency

## Key Dependencies
- `cpal` - Cross-platform audio I/O
- `tokio` - Async runtime for networking
- Network transport (TCP/UDP) for audio streaming

## Development Guidelines
- Keep latency minimal (< 50ms target)
- Handle audio device enumeration gracefully
- Implement proper error handling for device disconnections
- Use buffering to prevent audio dropouts
- Support common audio formats (16-bit PCM, 44.1kHz/48kHz)

## Commands
- Build: `cargo build --release`
- Run: `cargo run`
- Test: `cargo test`

## Testing
Test with various audio devices and network conditions to ensure robust operation.