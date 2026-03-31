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
