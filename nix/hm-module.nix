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
