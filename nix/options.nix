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
