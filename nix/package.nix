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
