{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.shadoword-desktop;
  system = pkgs.stdenv.hostPlatform.system;
  command = [ (lib.getExe cfg.package) ] ++ cfg.extraArgs;
in
{
  options.services.shadoword-desktop = {
    enable = lib.mkEnableOption "the Shadoword desktop client";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${system}.shadoword-desktop;
      defaultText = lib.literalExpression "inputs.shadoword.packages.<system>.shadoword-desktop";
      description = "Shadoword desktop package to install and run. Override to pin another package or version.";
    };

    autoStart = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether enabling the service adds the graphical session target as a startup dependency.";
    };

    environment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      description = "Additional environment variables for the desktop service.";
    };

    extraArgs = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "Additional arguments accepted by the Shadoword desktop executable.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    systemd.user.services.shadoword-desktop = {
      Unit = {
        Description = "Shadoword desktop speech-to-text client";
        Documentation = [ "https://github.com/Fractal-Tess/shadoword" ];
        After = [
          "graphical-session-pre.target"
          "pipewire.service"
        ];
        PartOf = [ "graphical-session.target" ];
      };
      Install.WantedBy = lib.optionals cfg.autoStart [ "graphical-session.target" ];

      Service = {
        Environment = lib.mapAttrsToList (name: value: "${name}=${value}") (
          {
            PATH = "${
              lib.makeBinPath [
                pkgs.xdotool
                pkgs.wtype
              ]
            }:%h/.nix-profile/bin:/etc/profiles/per-user/%u/bin:/run/current-system/sw/bin";
          }
          // cfg.environment
        );
        Type = "simple";
        ExecStart = lib.escapeShellArgs command;
        Restart = "on-failure";
        RestartSec = 2;
        TimeoutStopSec = 10;
      };
    };
  };
}
