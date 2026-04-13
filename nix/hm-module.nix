# Home-manager module for Shadoword speech-to-text
#
# Provides a systemd user service for autostart.
# Usage: imports = [ shadoword.homeManagerModules.default ];
#        services.shadoword.enable = true;
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.shadoword;
in
{
  options.services.shadoword = {
    enable = lib.mkEnableOption "Shadoword speech-to-text user service";

    package = lib.mkOption {
      type = lib.types.package;
      defaultText = lib.literalExpression "shadoword.packages.\${system}.shadoword";
      description = "The Shadoword package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.shadoword = {
      Unit = {
        Description = "Shadoword speech-to-text";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/shadoword-desktop";
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
