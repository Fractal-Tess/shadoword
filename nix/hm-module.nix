# Home-manager module for Shadow Word speech-to-text
#
# Provides a systemd user service for autostart.
# Usage: imports = [ shadowword.homeManagerModules.default ];
#        services.shadowword.enable = true;
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.shadowword;
in
{
  options.services.shadowword = {
    enable = lib.mkEnableOption "Shadow Word speech-to-text user service";

    package = lib.mkOption {
      type = lib.types.package;
      defaultText = lib.literalExpression "shadowword.packages.\${system}.shadowword";
      description = "The Shadow Word package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.shadowword = {
      Unit = {
        Description = "Shadow Word speech-to-text";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/shadowword";
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
