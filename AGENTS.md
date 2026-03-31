# Rsonance - AI Agent Guidelines

## What This Project Is

A Rust tool that streams microphone audio over TCP to a virtual audio input device. The transmitter captures mic input via `cpal` and sends S16LE PCM data; the receiver creates a PulseAudio `module-pipe-source` virtual microphone fed through a FIFO pipe.

Target use case: remote desktop audio forwarding.

## Environment

- **Nix + devenv**: the dev environment is managed by `devenv.nix` and `devenv.yaml`. There is no `flake.nix`. All environment changes (packages, env vars, toolchains) go in `devenv.nix`.
- **Rust edition 2024**, toolchain provided by devenv (rustc, cargo, clippy, rustfmt, rust-analyzer).
- **clang19 stdenv** for C/C++ compilation (needed by ALSA bindings).
- **System deps in devenv.nix**: `alsa-lib`, `pipewire`, `pulseaudio` (for `pactl`), `git`.
- **LIBCLANG_PATH** is set in `devenv.nix` for bindgen/cpal compilation.

## Architecture

```
Transmitter (async)                    Receiver (sync)
┌──────────────────┐   TCP (S16LE)    ┌──────────────────┐
│ cpal mic capture  │ ───────────────> │ std::net listener │
│ → convert_to_s16le│                  │ → FIFO pipe       │
│ → mpsc channel    │                  │ → PA pipe-source  │
│ → tokio TcpStream │                  │   (virtual mic)   │
└──────────────────┘                  └──────────────────┘
```

- **Transmitter** (`src/transmitter.rs`): async (tokio). Uses `cpal` callbacks bridged to async via `mpsc::unbounded_channel`. Audio converted to S16LE regardless of input format.
- **Receiver** (`src/receiver.rs`): synchronous (`std::net`, `std::thread`). Spawns thread per connection. Signal handling via `signal-hook` for clean PulseAudio cleanup on SIGINT.
- **Library** (`src/lib.rs`): shared utilities - PulseAudio virtual mic setup/cleanup (via `pactl` commands), address parsing, buffer validation.
- **Binary** (`src/main.rs`): CLI via `clap` with `receiver` and `transmitter` subcommands.

### Key Design Decisions

- Wire format is always S16LE at 44100Hz stereo, regardless of capture format.
- Virtual microphone uses PulseAudio `module-pipe-source` fed by a named FIFO (`mkfifo`).
- The receiver is intentionally synchronous - no async overhead for the simple TCP-to-FIFO pipeline.
- Audio sample conversion in `convert_to_s16le` uses `TypeId` runtime dispatch with unsafe pointer casts. This is pragmatic but not idiomatic Rust; if refactoring, consider trait-based dispatch.

## Development Commands

```bash
cargo build                                    # Debug build
cargo build --release                          # Release build
cargo test                                     # Run all tests (28 unit + 11 doc-tests)
cargo clippy --all-targets -- -D warnings      # Lint (must pass clean)
cargo fmt                                      # Format code
cargo fmt --check                              # Check formatting without changing
cargo run -- receiver [OPTIONS]                # Run receiver
cargo run -- transmitter [OPTIONS]             # Run transmitter
cargo run -- --help                            # CLI help
```

## Code Quality Rules

- **clippy must pass with `-D warnings`** - zero warnings allowed.
- **cargo fmt** must be applied before committing. Run `cargo fmt --check` to verify.
- **cargo test** must pass. Tests are inline `#[cfg(test)]` modules in each source file.
- Add dependencies with `cargo add <crate>`, not by editing Cargo.toml directly.

## Git Conventions

- **Atomic commits** - each commit should contain one logical change. Separate formatting, bug fixes, refactors, docs, and features into distinct commits.
- **Conventional commit messages** - use the `type(scope): description` format. Types: `feat`, `fix`, `chore`, `docs`, `style`, `refactor`, `test`.
- **Push after committing** unless there's a reason to hold (e.g. waiting for review).

## Project Structure

```
src/
├── main.rs          # CLI entry point (clap), dispatches to receiver/transmitter
├── lib.rs           # PulseAudio helpers, AudioConfig, address/buffer validation, tests
├── receiver.rs      # TCP listener → FIFO → virtual mic, signal handling, tests
└── transmitter.rs   # cpal capture → S16LE conversion → TCP stream, tests
```

No separate `tests/` directory - all tests are inline. No CI/CD configuration exists yet.

## Testing Notes

- PulseAudio-dependent tests (`test_setup_virtual_microphone_*`, `test_cleanup_*`, `test_get_virtual_microphone_module_id`) are smoke tests that accept any outcome since CI/dev environments may lack PulseAudio.
- FIFO tests in `receiver.rs` skip gracefully if `mkfifo` is unavailable.
- Transmitter conversion tests (`test_convert_*`) are proper unit tests with exact assertions.

## Platform Constraints

- **Linux only** - depends on PulseAudio/PipeWire and FIFO pipes.
- Requires `pactl` and `mkfifo` at runtime (provided by `pulseaudio` and coreutils in devenv).
- Microphone hardware required on transmitter machine for actual use (not for tests).
