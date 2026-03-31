# Nix Flake and Modules Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Nix flake that packages rsonance, exposes NixOS and Home Manager modules for the receiver service, and provides a devShell.

**Architecture:** Shared options in `nix/options.nix` consumed by both `nix/nixos-module.nix` and `nix/hm-module.nix`. Package built with `rustPlatform.buildRustPackage` in `nix/package.nix`. `flake.nix` wires everything together and adds a devShell. No flake-utils, no home-manager input.

**Tech Stack:** Nix flakes, rustPlatform.buildRustPackage, NixOS module system, Home Manager module system, systemd user services

---

### Task 1: Create the package derivation (`nix/package.nix`)

**Files:**
- Create: `nix/package.nix`

- [ ] **Step 1: Create `nix/` directory**

```bash
mkdir -p nix
```

- [ ] **Step 2: Write `nix/package.nix`**

```nix
{
  lib,
  rustPlatform,
  pkg-config,
  alsa-lib,
  libclang,
  makeWrapper,
  pulseaudio,
  coreutils,
}:

rustPlatform.buildRustPackage {
  pname = "rsonance";
  version = "0.1.0";

  src = lib.cleanSourceWith {
    src = ./..;
    filter =
      path: type:
      let
        baseName = builtins.baseNameOf path;
        parentDir = builtins.dirOf path;
      in
      (type == "directory" && (baseName == "src" || baseName == "nix"))
      || baseName == "Cargo.toml"
      || baseName == "Cargo.lock"
      || (builtins.baseNameOf parentDir == "src" && lib.hasSuffix ".rs" baseName);
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];

  buildInputs = [
    alsa-lib
  ];

  env.LIBCLANG_PATH = "${libclang.lib}/lib";

  postInstall = ''
    wrapProgram $out/bin/rsonance \
      --prefix PATH : ${lib.makeBinPath [ pulseaudio coreutils ]}
  '';

  meta = {
    description = "Audio transmission tool for remote microphone streaming";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "rsonance";
  };
}
```

- [ ] **Step 3: Commit**

```bash
git add nix/package.nix
git -c commit.gpgsign=false commit -m "feat(nix): add package derivation"
```

---

### Task 2: Create the shared options (`nix/options.nix`)

**Files:**
- Create: `nix/options.nix`

- [ ] **Step 1: Write `nix/options.nix`**

```nix
{ lib }:

let
  inherit (lib) mkOption mkEnableOption types;
in
{
  enable = mkEnableOption "the rsonance audio receiver service";

  host = mkOption {
    type = types.str;
    default = "0.0.0.0";
    description = "Host address for the receiver to bind to.";
  };

  port = mkOption {
    type = types.port;
    default = 8080;
    description = "Port for the receiver to listen on.";
  };

  bufferSize = mkOption {
    type = types.int;
    default = 4096;
    description = "Audio buffer size in bytes. Affects latency.";
  };

  microphoneName = mkOption {
    type = types.str;
    default = "rsonance_virtual_microphone";
    description = "Name of the PulseAudio virtual microphone.";
  };

  fifoPath = mkOption {
    type = types.str;
    default = "/tmp/rsonance_audio_pipe";
    description = "Path to the FIFO pipe for audio data.";
  };

  verbose = mkOption {
    type = types.bool;
    default = false;
    description = "Enable verbose logging output.";
  };
}
```

- [ ] **Step 2: Commit**

```bash
git add nix/options.nix
git -c commit.gpgsign=false commit -m "feat(nix): add shared module options"
```

---

### Task 3: Create the NixOS module (`nix/nixos-module.nix`)

**Files:**
- Create: `nix/nixos-module.nix`

- [ ] **Step 1: Write `nix/nixos-module.nix`**

```nix
self:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  inherit (lib) mkIf mkOption types;
  sharedOptions = import ./options.nix { inherit lib; };
  cfg = config.services.rsonance;
in
{
  options.services.rsonance = sharedOptions // {
    package = mkOption {
      type = types.package;
      default = self.packages.${pkgs.system}.default;
      description = "The rsonance package to use.";
    };
  };

  config = mkIf cfg.enable {
    systemd.user.services.rsonance = {
      description = "Rsonance audio receiver";
      after = [
        "pulseaudio.service"
        "pipewire-pulse.service"
      ];
      wants = [
        "pulseaudio.service"
        "pipewire-pulse.service"
      ];
      wantedBy = [ "default.target" ];
      serviceConfig = {
        Type = "simple";
        ExecStart = builtins.concatStringsSep " " (
          [
            "${cfg.package}/bin/rsonance"
            "receiver"
            "--host"
            cfg.host
            "--port"
            (toString cfg.port)
            "--buffer-size"
            (toString cfg.bufferSize)
            "--microphone-name"
            cfg.microphoneName
            "--fifo-path"
            cfg.fifoPath
          ]
          ++ lib.optional cfg.verbose "--verbose"
        );
        Restart = "no";
      };
    };
  };
}
```

- [ ] **Step 2: Commit**

```bash
git add nix/nixos-module.nix
git -c commit.gpgsign=false commit -m "feat(nix): add NixOS module for receiver service"
```

---

### Task 4: Create the Home Manager module (`nix/hm-module.nix`)

**Files:**
- Create: `nix/hm-module.nix`

- [ ] **Step 1: Write `nix/hm-module.nix`**

```nix
self:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  inherit (lib) mkIf mkOption types;
  sharedOptions = import ./options.nix { inherit lib; };
  cfg = config.services.rsonance;
in
{
  options.services.rsonance = sharedOptions // {
    package = mkOption {
      type = types.package;
      default = self.packages.${pkgs.system}.default;
      description = "The rsonance package to use.";
    };
  };

  config = mkIf cfg.enable {
    systemd.user.services.rsonance = {
      Unit = {
        Description = "Rsonance audio receiver";
        After = [
          "pulseaudio.service"
          "pipewire-pulse.service"
        ];
        Wants = [
          "pulseaudio.service"
          "pipewire-pulse.service"
        ];
      };
      Service = {
        Type = "simple";
        ExecStart = builtins.concatStringsSep " " (
          [
            "${cfg.package}/bin/rsonance"
            "receiver"
            "--host"
            cfg.host
            "--port"
            (toString cfg.port)
            "--buffer-size"
            (toString cfg.bufferSize)
            "--microphone-name"
            cfg.microphoneName
            "--fifo-path"
            cfg.fifoPath
          ]
          ++ lib.optional cfg.verbose "--verbose"
        );
        Restart = "no";
      };
      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
```

Note: Home Manager uses a different service definition format than NixOS. NixOS uses `serviceConfig` with lowercase unit section keys managed by the module system. Home Manager uses the INI-style `Unit`/`Service`/`Install` attribute sets directly.

- [ ] **Step 2: Commit**

```bash
git add nix/hm-module.nix
git -c commit.gpgsign=false commit -m "feat(nix): add Home Manager module for receiver service"
```

---

### Task 5: Create the flake (`flake.nix`)

**Files:**
- Create: `flake.nix`

- [ ] **Step 1: Write `flake.nix`**

```nix
{
  description = "Rsonance - Audio transmission tool for remote microphone streaming";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      packages = forAllSystems (system: {
        default = (pkgsFor system).callPackage ./nix/package.nix { };
      });

      nixosModules.default = import ./nix/nixos-module.nix self;

      homeManagerModules.default = import ./nix/hm-module.nix self;

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = (pkgs.mkShell.override { stdenv = pkgs.clang19Stdenv; }) {
            packages = with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              rust-analyzer
              pkg-config
              alsa-lib
              libclang
              pipewire
              pulseaudio
              git
            ];

            env = {
              LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            };
          };
        }
      );
    };
}
```

- [ ] **Step 2: Generate `flake.lock`**

```bash
nix flake lock
```

This creates `flake.lock` by resolving the nixpkgs input.

- [ ] **Step 3: Commit**

```bash
git add flake.nix flake.lock
git -c commit.gpgsign=false commit -m "feat(nix): add flake with package, modules, and devShell"
```

---

### Task 6: Update `.gitignore`

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Add Nix build result to `.gitignore`**

Add the following line to `.gitignore`:

```
# Nix
result
```

- [ ] **Step 2: Commit**

```bash
git add .gitignore
git -c commit.gpgsign=false commit -m "chore: add nix build result to gitignore"
```

---

### Task 7: Verify the build

**Files:**
- None (verification only)

- [ ] **Step 1: Check flake outputs are well-formed**

```bash
nix flake check
```

Expected: no errors. This validates the flake structure and evaluates the derivations.

- [ ] **Step 2: Build the package**

```bash
nix build
```

Expected: builds successfully, produces `result/bin/rsonance`.

- [ ] **Step 3: Verify the wrapper works**

```bash
./result/bin/rsonance --help
```

Expected: prints the help text with `receiver` and `transmitter` subcommands. This confirms the binary runs and the wrapper didn't break anything.

- [ ] **Step 4: Verify runtime deps are wrapped**

```bash
ldd ./result/bin/rsonance 2>/dev/null; readlink -f ./result/bin/rsonance
```

Check that the binary at `result/bin/rsonance` is a wrapper script (shell script, not ELF). Inspect it:

```bash
cat ./result/bin/rsonance | head -5
```

Expected: a wrapper script that sets `PATH` to include `pactl` and `mkfifo`.

- [ ] **Step 5: Test the devShell enters**

```bash
nix develop --command bash -c "cargo --version && rustc --version && pactl --version"
```

Expected: prints cargo, rustc, and pactl versions without errors.

- [ ] **Step 6: Evaluate NixOS module (smoke test)**

```bash
nix eval --json .#nixosModules.default --apply 'x: builtins.typeOf x'
```

Expected: `"lambda"` — confirms the module is a function.

- [ ] **Step 7: Evaluate Home Manager module (smoke test)**

```bash
nix eval --json .#homeManagerModules.default --apply 'x: builtins.typeOf x'
```

Expected: `"lambda"` — confirms the module is a function.
