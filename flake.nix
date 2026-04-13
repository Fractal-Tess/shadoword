{
  description = "Shadoword - Rust workspace for the egui desktop client and Whisper daemon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      workspaceToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = workspaceToml.workspace.package.version;

      mkPkgs = system: import nixpkgs {
        inherit system;
        config.allowUnfree = true;
      };

      commonBuildDeps = pkgs:
        with pkgs; [
          cmake
          glslang
          llvmPackages.libclang
          makeWrapper
          pkg-config
          shaderc
          vulkan-headers
          vulkan-loader
          vulkan-tools
        ];

      daemonRuntimeDeps = pkgs:
        with pkgs; [
          alsa-lib
          libglvnd
          openssl
          vulkan-loader
        ];

      desktopRuntimeDeps = pkgs:
        daemonRuntimeDeps pkgs
        ++ (with pkgs; [
          fontconfig
          libevdev
          libx11
          libxcb
          libxkbcommon
          wayland
        ]);

      commonEnv = pkgs: {
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.lib.getVersion pkgs.llvmPackages.libclang}/include -isystem ${pkgs.glibc.dev}/include";
        VULKAN_SDK = "${pkgs.vulkan-headers}";
      };

      runtimeLibraryPath = pkgs: runtimeDeps:
        "/run/opengl-driver/lib:${pkgs.lib.makeLibraryPath runtimeDeps}";

      mkRustPackage =
        {
          pkgs,
          system,
          pname,
          cargoPackage,
          runtimeDeps,
        }:
        pkgs.rustPlatform.buildRustPackage {
          inherit pname version;
          src = self;

          cargoLock.lockFile = ./Cargo.lock;

          cargoBuildFlags =
            [
              "-p"
              cargoPackage
            ]
            ++ pkgs.lib.optionals (system == "x86_64-linux") [
              "--features"
              "whisper-vulkan"
            ];

          nativeBuildInputs = commonBuildDeps pkgs;
          buildInputs = runtimeDeps;

          env = commonEnv pkgs;
          doCheck = false;

          postInstall = ''
            wrapProgram "$out/bin/${cargoPackage}" \
              --prefix LD_LIBRARY_PATH : "${runtimeLibraryPath pkgs runtimeDeps}"
          '';

          meta = {
            description = "Offline speech-to-text workspace with egui desktop UI and Whisper daemon";
            homepage = "https://github.com/Fractal-Tess/shadoword";
            license = pkgs.lib.licenses.mit;
            platforms = supportedSystems;
          };
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
        in
        {
          default = mkRustPackage {
            inherit pkgs;
            inherit system;
            pname = "shadoword";
            cargoPackage = "shadoword-desktop";
            runtimeDeps = desktopRuntimeDeps pkgs;
          };

          shadoword = mkRustPackage {
            inherit pkgs;
            inherit system;
            pname = "shadoword";
            cargoPackage = "shadoword-desktop";
            runtimeDeps = desktopRuntimeDeps pkgs;
          };

          shadoword-desktop = mkRustPackage {
            inherit pkgs;
            inherit system;
            pname = "shadoword-desktop";
            cargoPackage = "shadoword-desktop";
            runtimeDeps = desktopRuntimeDeps pkgs;
          };

          shadoword-daemon = mkRustPackage {
            inherit pkgs;
            inherit system;
            pname = "shadoword-daemon";
            cargoPackage = "shadoword-daemon";
            runtimeDeps = daemonRuntimeDeps pkgs;
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          runtimeDeps = desktopRuntimeDeps pkgs;
        in
        {
          default = pkgs.mkShell {
            buildInputs =
              commonBuildDeps pkgs
              ++ runtimeDeps
              ++ (with pkgs; [
                cargo
                clippy
                rust-analyzer
                rustc
              ]);

            inherit (commonEnv pkgs)
              LIBCLANG_PATH
              BINDGEN_EXTRA_CLANG_ARGS
              VULKAN_SDK;

            LD_LIBRARY_PATH = runtimeLibraryPath pkgs runtimeDeps;

            shellHook = ''
              export VK_DRIVER_FILES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.x86_64.json
              export VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.x86_64.json
              export VK_LAYER_PATH=/run/opengl-driver/share/vulkan/implicit_layer.d:/run/opengl-driver/share/vulkan/explicit_layer.d
              if [ -n "''${XDG_DATA_DIRS:-}" ]; then
                export XDG_DATA_DIRS=/run/opengl-driver/share:$XDG_DATA_DIRS
              else
                export XDG_DATA_DIRS=/run/opengl-driver/share
              fi
              echo "Shadoword Whisper development environment"
              echo "Run 'cargo run -p shadoword-desktop --features whisper-vulkan' for the egui app"
              echo "Run 'cargo run -p shadoword-daemon --features whisper-vulkan' for the daemon"
            '';
          };
        }
      );
    };
}
