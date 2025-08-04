# Mike - Audio Transmission Tool

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://rustup.rs/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A high-performance Rust tool that captures microphone audio and transmits it to another device for playback through a virtual audio input device. This enables remote desktop software to capture audio from a remote microphone with minimal latency.

## 🎯 Key Features

- **Virtual Audio Input**: Creates a virtual microphone that appears as a real input device in your system
- **Low Latency**: Optimized for real-time audio transmission with configurable buffer sizes  
- **Auto-Reconnection**: Transmitter automatically reconnects if connection drops
- **Multiple Audio Formats**: Supports F32, I16, and U16 audio sample formats with automatic conversion
- **PulseAudio Compatible**: Works seamlessly with PulseAudio and PipeWire (via PA compatibility)
- **Clean Exit**: Automatically removes virtual device and cleans up resources on exit (Ctrl+C)
- **Cross-Platform Audio**: Uses CPAL for cross-platform audio I/O

## 🏗️ Architecture

- **Transmitter**: Captures microphone audio using `cpal` and streams via TCP
- **Receiver**: Creates virtual microphone input and pipes received audio through FIFO
- **Core Library**: Modular design with comprehensive error handling and testing

## 🚀 Quick Start

### 1. Start the Receiver (Virtual Microphone)
On the remote desktop machine:
```bash
cargo run -- receiver
```

This creates a virtual microphone device at the default address `0.0.0.0:8080`.

### 2. Start the Transmitter (Microphone Capture)  
On the local machine with the microphone:
```bash
cargo run -- transmitter
```

This connects to `127.0.0.1:8080` by default and starts streaming audio.

### 3. Configure Remote Desktop Software
In your remote desktop application, select "mike_virtual_microphone" as the microphone input device.

## 📖 Usage Examples

### Basic Usage
```bash
# Start receiver with defaults
cargo run -- receiver

# Start transmitter with defaults  
cargo run -- transmitter
```

### Custom Configuration
```bash
# Receiver with custom settings
cargo run -- receiver --host 192.168.1.100 --port 9090 --buffer-size 8192 --verbose

# Transmitter with custom settings
cargo run -- transmitter --host 192.168.1.100 --port 9090 --reconnect-attempts 10 --verbose
```

### Advanced Options
```bash
# Custom virtual microphone name and FIFO path
cargo run -- receiver \
  --microphone-name "my_remote_mic" \
  --fifo-path "/tmp/my_audio_pipe" \
  --buffer-size 2048

# High-reliability transmitter
cargo run -- transmitter \
  --host 192.168.1.100 \
  --reconnect-attempts 20 \
  --buffer-size 1024 \
  --verbose
```

## ⚙️ Configuration Options

### Receiver Options
| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--host` | `-H` | `0.0.0.0` | Host address to bind to |
| `--port` | `-p` | `8080` | Port to listen on |
| `--buffer-size` | `-b` | `4096` | Audio buffer size in bytes |
| `--microphone-name` | `-m` | `mike_virtual_microphone` | Virtual microphone name |
| `--fifo-path` | `-f` | `/tmp/mike_audio_pipe` | FIFO pipe path |
| `--verbose` | `-v` | `false` | Enable verbose output |

### Transmitter Options
| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--host` | `-H` | `127.0.0.1` | Server address to connect to |
| `--port` | `-p` | `8080` | Server port to connect to |
| `--buffer-size` | `-b` | `4096` | Audio buffer size in bytes |
| `--reconnect-attempts` | `-r` | `5` | Max reconnection attempts |
| `--verbose` | `-v` | `false` | Enable verbose output |

### Buffer Size Guidelines
- **1024 bytes**: Very low latency, may cause dropouts on slower systems
- **4096 bytes**: Good balance of latency and stability (default)
- **8192 bytes**: Higher latency but very stable
- **16384 bytes**: High latency, maximum stability

## 🔧 Development

### Building
```bash
# Debug build
cargo build

# Release build  
cargo build --release

# Build all targets
cargo build --all-targets
```

### Testing
```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_audio_config_default
```

### Code Quality
```bash
# Lint with clippy
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

## 🧪 Testing

The project includes comprehensive test coverage:
- **28 unit tests** covering core functionality
- **11 doc tests** ensuring documentation examples work
- **Integration tests** for virtual microphone operations
- **Audio conversion tests** for all supported formats
- **Error handling tests** for network and system failures

## 📋 System Requirements

- **Operating System**: Linux with PulseAudio or PipeWire
- **Audio System**: PulseAudio or PipeWire with PulseAudio compatibility
- **Commands**: `pactl` and `mkfifo` must be available in PATH
- **Network**: TCP connectivity between transmitter and receiver machines
- **Hardware**: Microphone on transmitter machine

## 🛠️ Dependencies

- **cpal**: Cross-platform audio I/O library
- **tokio**: Async runtime for networking  
- **anyhow**: Error handling
- **clap**: Command-line argument parsing
- **signal-hook**: Signal handling for clean shutdown

## 🔍 Troubleshooting

### Audio Issues
```bash
# Check if PulseAudio/PipeWire is running
pactl info

# List available audio sources
pactl list sources short

# Verify virtual microphone exists
pactl list sources short | grep mike
```

### Network Issues
- Ensure firewall allows TCP connections on the configured port
- Verify network connectivity between transmitter and receiver
- Check if the port is already in use: `netstat -ln | grep :8080`

### Manual Cleanup
If the virtual microphone isn't cleaned up properly:
```bash
# Find the module ID
pactl list modules short | grep pipe-source

# Unload the module (replace <id> with actual module ID)
pactl unload-module <id>

# Remove FIFO pipe if it exists
rm -f /tmp/mike_audio_pipe
```

### Common Issues
- **"No input device available"**: Ensure a microphone is connected to the transmitter machine
- **"FIFO pipe does not exist"**: The receiver creates the FIFO; ensure it started successfully
- **Connection refused**: Check that receiver is running and firewall allows connections
- **Audio stuttering**: Try increasing buffer size or check network stability

## 🏃‍♂️ Performance Tips

1. **Reduce Latency**: Use smaller buffer sizes (1024-2048 bytes)
2. **Improve Stability**: Use larger buffer sizes (8192+ bytes) 
3. **Network Optimization**: Use wired connections when possible
4. **System Tuning**: Ensure audio system isn't under heavy load

## 🤝 Contributing

Contributions are welcome! Please ensure:
- Code passes `cargo clippy --all-targets -- -D warnings`
- All tests pass with `cargo test`
- Documentation is updated for new features
- New functionality includes appropriate tests

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [CPAL](https://github.com/RustAudio/cpal) for cross-platform audio
- Uses [Tokio](https://tokio.rs/) for async networking
- Inspired by the need for seamless remote audio streaming