# Rsonance - Audio Transmission Tool

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://rustup.rs/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

A Rust tool that captures microphone audio and streams it to create a virtual microphone on another device. Enables remote desktop software to capture audio from remote microphones with low latency.

## Usage

```bash
# Start receiver (creates virtual microphone)
cargo run -- receiver

# Start transmitter (captures and streams audio)
cargo run -- transmitter --host <receiver_ip>

# Help
cargo run -- --help
```

## Requirements

- Linux with PulseAudio/PipeWire
- Network connectivity between machines

## License

Licensed under Apache 2.0 - see [LICENSE](LICENSE) for details.

Developed with [Claude Code](https://claude.ai/code)