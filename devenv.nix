{pkgs, ...}: {
  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    alsa-lib
  ];

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer"];
  };
}
