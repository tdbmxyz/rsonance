{pkgs, ...}: {
  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    alsa-lib
    pipewire
  ];

  stdenv = pkgs.clang19Stdenv;

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer"];
  };

  env = {
    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
  };
}
