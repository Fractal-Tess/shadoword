# NixOS module for the Shadoword transcription daemon.
#
# Takes the flake's `self` so packages come from this repository's own nixpkgs
# evaluation. That keeps `allowUnfree` and the CUDA toolchain out of the host's
# nixpkgs config: enabling the CUDA variant should not force every other package
# on the machine through an unfree evaluation.
self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.shadoword-api;
  inherit (lib)
    mkIf
    mkOption
    types
    optional
    optionals
    optionalAttrs
    escapeShellArg
    ;

  flakePackages = self.packages.${pkgs.stdenv.hostPlatform.system};
  variantPackages = {
    cpu = flakePackages.shadoword-api;
    cuda = flakePackages.shadoword-api-cuda;
    vulkan = flakePackages.shadoword-api-vulkan;
  };

  listenEndpoint = "${cfg.listenAddress}:${toString cfg.port}";
  needsGpu = cfg.variant != "cpu";

  # The daemon and the token CLI have to agree on which file they are editing, and
  # that file is owned by the service user. A bare `shadoword-api token generate`
  # run by root would silently write to /root/.config instead, so the copy on PATH
  # pins the service's config path.
  cliWrapper = pkgs.writeShellScriptBin "shadoword-api" ''
    exec ${cfg.package}/bin/shadoword-api --config ${escapeShellArg cfg.configFile} "$@"
  '';

  writablePaths = [
    cfg.stateDir
    cfg.modelDir
    (builtins.dirOf cfg.configFile)
  ]
  ++ optional (cfg.requestRecordingDir != null) cfg.requestRecordingDir;
in
{
  options.services.shadoword-api = {
    enable = lib.mkEnableOption "the Shadoword speech-to-text daemon";

    variant = mkOption {
      type = types.enum [
        "cpu"
        "cuda"
        "vulkan"
      ];
      default = "cpu";
      description = ''
        Inference backend to run. `cuda` and `vulkan` build the daemon with GPU
        acceleration and grant it the `render` and `video` groups.
      '';
    };

    package = mkOption {
      type = types.package;
      default = variantPackages.${cfg.variant};
      defaultText = lib.literalMD "the `shadoword-api` build matching {option}`services.shadoword-api.variant`";
      description = "Shadoword API package to run. Overriding this ignores {option}`variant`.";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      example = "0.0.0.0";
      description = "Address the daemon binds to.";
    };

    port = mkOption {
      type = types.port;
      default = 47813;
      description = "TCP port the daemon listens on.";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Open {option}`port` on every interface. Leave this off and use
        {option}`networking.firewall.interfaces` to expose the daemon on a single
        trusted network instead.
      '';
    };

    stateDir = mkOption {
      type = types.path;
      default = "/var/lib/shadoword";
      description = "Directory holding the daemon's configuration, models, and cache.";
    };

    configFile = mkOption {
      type = types.path;
      default = "${cfg.stateDir}/config/shadoword/api.json";
      defaultText = lib.literalExpression ''"''${config.services.shadoword-api.stateDir}/config/shadoword/api.json"'';
      description = ''
        Mutable configuration owned by the daemon, including hashed token records.
        It is written by the daemon and by `shadoword-api token`, so it cannot live
        in the Nix store.
      '';
    };

    modelDir = mkOption {
      type = types.path;
      default = "${cfg.stateDir}/models";
      defaultText = lib.literalExpression ''"''${config.services.shadoword-api.stateDir}/models"'';
      description = "Directory Whisper model files are downloaded to.";
    };

    downloadModels = mkOption {
      type = types.listOf types.str;
      default = [ "turbo" ];
      example = [
        "turbo"
        "small"
      ];
      description = ''
        Catalog models fetched into {option}`modelDir` before the daemon starts.
        Which one is active stays an API-managed runtime setting.
      '';
    };

    initTokenFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      example = lib.literalExpression "config.sops.secrets.shadoword_admin_token.path";
      description = ''
        File holding an admin token to install on first start. It is passed through
        systemd credentials rather than the environment, and is only adopted while
        the daemon has no tokens of its own — so rotating the secret means revoking
        the old token, not just editing this file.

        Without it, issue the first token by hand:
        `sudo -u shadoword shadoword-api token generate admin <name>`.
      '';
    };

    queueCapacity = mkOption {
      type = types.nullOr types.ints.positive;
      default = null;
      description = "Number of queued transcription requests. Null keeps the daemon's own default.";
    };

    requestRecordingDir = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Archive every accepted request as WAV plus response metadata into this directory.";
    };

    logLevel = mkOption {
      type = types.str;
      default = "info";
      description = "Value for `RUST_LOG`.";
    };

    environment = mkOption {
      type = types.attrsOf types.str;
      default = { };
      description = "Extra environment variables for the service unit.";
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [ ];
      example = [ "--no-preload" ];
      description = "Additional arguments appended to the daemon command line.";
    };
  };

  config = mkIf cfg.enable {
    users.groups.shadoword = { };
    users.users.shadoword = {
      isSystemUser = true;
      group = "shadoword";
      home = cfg.stateDir;
    };

    environment.systemPackages = [ cliWrapper ];

    networking.firewall.allowedTCPPorts = optional cfg.openFirewall cfg.port;

    systemd.tmpfiles.rules = map (path: "d ${path} 0700 shadoword shadoword -") writablePaths;

    systemd.services.shadoword-api = {
      description = "Shadoword speech-to-text daemon (${cfg.variant})";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      environment = {
        RUST_LOG = cfg.logLevel;
        XDG_CONFIG_HOME = "${cfg.stateDir}/config";
        XDG_DATA_HOME = "${cfg.stateDir}/data";
      }
      // optionalAttrs (cfg.initTokenFile != null) {
        SHADOWORD_INIT_TOKEN_FILE = "%d/init-token";
      }
      // optionalAttrs (cfg.requestRecordingDir != null) {
        SHADOWORD_REQUEST_RECORDING_DIR = cfg.requestRecordingDir;
      }
      // cfg.environment;

      serviceConfig = {
        Type = "simple";
        User = "shadoword";
        Group = "shadoword";
        SupplementaryGroups = optionals needsGpu [
          "render"
          "video"
        ];

        ExecStart = lib.concatStringsSep " " (
          [
            "${cfg.package}/bin/shadoword-api"
            "--config"
            (escapeShellArg cfg.configFile)
            "--listen"
            (escapeShellArg listenEndpoint)
            "--download-dir"
            (escapeShellArg cfg.modelDir)
          ]
          ++ lib.concatMap (model: [
            "--download-model"
            (escapeShellArg model)
          ]) cfg.downloadModels
          ++ optionals (cfg.queueCapacity != null) [
            "--queue-capacity"
            (toString cfg.queueCapacity)
          ]
          ++ map escapeShellArg cfg.extraArgs
        );

        LoadCredential = optional (cfg.initTokenFile != null) "init-token:${cfg.initTokenFile}";

        Restart = "on-failure";
        RestartSec = "5s";
        # First boot downloads a model before the daemon can listen.
        TimeoutStartSec = "30min";
        TimeoutStopSec = "30s";

        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectHome = true;
        ProtectSystem = "strict";
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectProc = "invisible";
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        RemoveIPC = true;
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_INET6"
          "AF_UNIX"
        ];
        SystemCallArchitectures = "native";
        # `@resources` stays allowed: ggml pins worker threads, and the CUDA
        # runtime sets NUMA memory policy.
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
        ];
        # GPU builds need the real /dev to reach their render node.
        PrivateDevices = !needsGpu;
        ReadWritePaths = writablePaths;
      };
    };
  };
}
