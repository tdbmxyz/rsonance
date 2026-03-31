# Nix Flake and Modules Design

## Goal

Add a Nix flake to rsonance that provides:
1. A package derivation built with `rustPlatform.buildRustPackage`
2. A NixOS module exposing the receiver as a systemd user service
3. A Home Manager module exposing the same
4. A devShell replacing the current devenv setup for `nix develop` users

## File Structure

```
flake.nix                  # Flake entry point
flake.lock                 # Auto-generated
nix/
  package.nix              # rustPlatform.buildRustPackage derivation
  options.nix              # Shared option declarations
  nixos-module.nix         # NixOS module
  hm-module.nix            # Home Manager module
```

Existing `devenv.nix` and `devenv.yaml` remain unchanged.

## Flake Inputs

- `nixpkgs` — the only input. No `home-manager`, no `flake-utils`.

System support: `x86_64-linux` and `aarch64-linux` only (Linux-only project). Use `nixpkgs.lib.genAttrs` to iterate over supported systems.

## Package (`nix/package.nix`)

A function taking `{ lib, rustPlatform, pkg-config, alsa-lib, libclang, makeWrapper, pulseaudio, coreutils }`.

- Builder: `rustPlatform.buildRustPackage`
- `cargoLock.lockFile` pointing to `../Cargo.lock`
- `src` filtered to include only Rust source and Cargo files
- Native build inputs: `pkg-config`
- Build inputs: `alsa-lib`
- `LIBCLANG_PATH` set to `"${libclang.lib}/lib"` as a build environment variable
- Post-install: `wrapProgram` to add `pulseaudio` (for `pactl`) and `coreutils` (for `mkfifo`) to `PATH`
- Meta: license, description, platforms limited to Linux

## Shared Options (`nix/options.nix`)

A function taking `{ lib }` and returning an attribute set of NixOS module options. This file declares all options *except* `package`, since the package default depends on `self` and `pkgs.system` which are only available in the module context. Each module adds the `package` option with the appropriate default.

All options live under the `services.rsonance` namespace:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | `bool` | `false` | Enable the rsonance receiver service |
| `host` | `str` | `"0.0.0.0"` | Host address to bind to |
| `port` | `port` | `8080` | Port to listen on |
| `bufferSize` | `int` | `4096` | Audio buffer size in bytes |
| `microphoneName` | `str` | `"rsonance_virtual_microphone"` | Virtual microphone name in PulseAudio |
| `fifoPath` | `str` | `"/tmp/rsonance_audio_pipe"` | FIFO pipe path for audio data |
| `verbose` | `bool` | `false` | Enable verbose logging |
| `package` | `package` | `self.packages.<system>.default` | The rsonance package to use (defined per-module, not in options.nix) |

## NixOS Module (`nix/nixos-module.nix`)

A function taking the flake's `self` as a parameter and returning a standard NixOS module `{ config, lib, pkgs, ... }`.

- Imports `options.nix` for option declarations
- Sets `options.services.rsonance` to the shared options (with `package` defaulting to `self.packages.${pkgs.system}.default`)
- When `cfg.enable` is true, defines `config.systemd.user.services.rsonance`:

```ini
[Unit]
Description=Rsonance audio receiver
After=pulseaudio.service pipewire-pulse.service
Wants=pulseaudio.service pipewire-pulse.service

[Service]
Type=simple
ExecStart=${cfg.package}/bin/rsonance receiver \
  --host ${cfg.host} \
  --port ${toString cfg.port} \
  --buffer-size ${toString cfg.bufferSize} \
  --microphone-name ${cfg.microphoneName} \
  --fifo-path ${cfg.fifoPath} \
  ${lib.optionalString cfg.verbose "--verbose"}
Restart=no

[Install]
WantedBy=default.target
```

## Home Manager Module (`nix/hm-module.nix`)

Same structure as the NixOS module but targeting Home Manager's module system.

- Same shared options under `options.services.rsonance`
- When `cfg.enable` is true, defines `config.systemd.user.services.rsonance` (Home Manager's native interface for user services)
- Identical service definition to the NixOS module

The key difference is the evaluation context: HM modules are evaluated within Home Manager's module system, not NixOS's. The service attribute path (`systemd.user.services`) is the same in both, but HM handles it through its own systemd integration.

## Flake Outputs

```nix
{
  packages.<system>.default    = rsonance package
  nixosModules.default         = NixOS module
  homeManagerModules.default   = Home Manager module
  devShells.<system>.default   = development shell
}
```

## DevShell

`mkShell.override { stdenv = pkgs.clang19Stdenv; }` providing:

- **Rust toolchain**: `cargo`, `rustc`, `clippy`, `rustfmt`, `rust-analyzer` (from nixpkgs)
- **Build deps**: `alsa-lib`, `pkg-config`, `libclang`
- **Runtime deps**: `pipewire`, `pulseaudio`
- **Utilities**: `git`
- **Env vars**: `LIBCLANG_PATH = "${pkgs.libclang.lib}/lib"`

This mirrors the current `devenv.nix` configuration exactly.

## Usage Examples

### NixOS (`configuration.nix` or flake module)

```nix
{
  imports = [ rsonance.nixosModules.default ];

  services.rsonance = {
    enable = true;
    port = 9090;
    verbose = true;
  };
}
```

### Home Manager (`home.nix` or flake module)

```nix
{
  imports = [ rsonance.homeManagerModules.default ];

  services.rsonance = {
    enable = true;
    host = "192.168.1.100";
    microphoneName = "my_remote_mic";
  };
}
```

### Development

```bash
nix develop    # enters devShell with full Rust toolchain + deps
```

## What This Does NOT Include

- No transmitter service — the transmitter is run ad-hoc on the client machine, not as a persistent service.
- No CI/CD — out of scope.
- No NixOS integration tests (`nixosTest`) — the receiver requires PulseAudio at runtime which is difficult to test in a sandboxed NixOS test VM. Could be added later.
- No auto-restart on failure — per design decision.
