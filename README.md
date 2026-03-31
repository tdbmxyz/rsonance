# Rsonance

[![Rust](https://img.shields.io/badge/rust-2024_edition-brightgreen.svg)](https://rustup.rs/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Stream microphone audio over TCP to a virtual audio input device. Designed for remote desktop audio forwarding where the host machine needs access to the client's microphone.

## How It Works

```
Local machine                          Remote desktop host
┌──────────────────┐   TCP (S16LE)    ┌──────────────────┐
│   Transmitter     │ ───────────────> │     Receiver      │
│                   │                  │                   │
│  Mic → S16LE PCM  │                  │  TCP → FIFO pipe  │
│  → TCP stream     │                  │  → PulseAudio     │
│                   │                  │    virtual mic     │
└──────────────────┘                  └──────────────────┘
```

The transmitter captures microphone input via [cpal](https://github.com/RustAudio/cpal), converts all sample formats to S16LE, and sends raw PCM over TCP. The receiver writes incoming audio to a FIFO pipe that feeds a PulseAudio `module-pipe-source` virtual microphone.

## Requirements

- **Linux** with PulseAudio or PipeWire (with PulseAudio compatibility layer)
- **Nix** with [devenv](https://devenv.sh) (manages all build dependencies)
- Microphone on the transmitter machine

## Setup

```bash
# Clone and enter the dev environment
git clone https://github.com/tdbmxyz/rsonance.git
cd rsonance

# devenv activates automatically via direnv, or manually:
devenv shell
```

The devenv provides: Rust toolchain, clang19 (for ALSA bindings), alsa-lib, pipewire, pulseaudio (pactl), and git.

## Usage

```bash
# On the remote desktop host - start the receiver (creates virtual microphone)
cargo run -- receiver

# On the local machine - stream microphone to the receiver
cargo run -- transmitter --host <receiver_ip>

# See all options
cargo run -- --help
cargo run -- receiver --help
cargo run -- transmitter --help
```

### Receiver Options

| Flag | Default | Description |
|------|---------|-------------|
| `-H, --host` | `0.0.0.0` | Bind address |
| `-p, --port` | `8080` | Listen port |
| `-b, --buffer-size` | `4096` | Buffer size in bytes |
| `-m, --microphone-name` | `rsonance_virtual_microphone` | Virtual mic name |
| `-f, --fifo-path` | `/tmp/rsonance_audio_pipe` | FIFO pipe path |
| `-v, --verbose` | off | Verbose output |

### Transmitter Options

| Flag | Default | Description |
|------|---------|-------------|
| `-H, --host` | `127.0.0.1` | Server address |
| `-p, --port` | `8080` | Server port |
| `-b, --buffer-size` | `4096` | Buffer size in bytes |
| `-r, --reconnect-attempts` | `5` | Max reconnection attempts |
| `-v, --verbose` | off | Verbose output |

Once running, select the virtual microphone (e.g. "Rsonance Virtual Microphone") as the audio input in your remote desktop application.

## Development

```bash
cargo build                                    # Debug build
cargo test                                     # Run tests
cargo clippy --all-targets -- -D warnings      # Lint (must pass clean)
cargo fmt --check                              # Check formatting
```

### Logging

Control log verbosity via `RUST_LOG`:

```bash
RUST_LOG=debug cargo run -- receiver --verbose
RUST_LOG=warn cargo run -- transmitter
```

### Troubleshooting

```bash
pactl info                                     # Check PulseAudio is running
pactl list sources short                       # List audio sources
pactl list modules short | grep pipe-source    # Find virtual mic module
pactl unload-module <id>                       # Manual cleanup if needed
```

## License

Licensed under Apache 2.0 - see [LICENSE](LICENSE) for details.
