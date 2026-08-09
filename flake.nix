{
  description = "Shadoword - Tauri desktop client and Whisper daemon";

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

      workspaceToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = workspaceToml.workspace.package.version;
      minimumRustVersion = workspaceToml.workspace.package."rust-version";
      releaseArtifacts = builtins.fromJSON (builtins.readFile ./nix/release-artifacts.json);

      artifactFor =
        system: packageName:
        if releaseArtifacts.version == version then
          releaseArtifacts.systems.${system}.${packageName} or null
        else
          null;

      mkPkgs =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config.allowUnfree = true;
          };
        in
        if pkgs.lib.versionAtLeast pkgs.rustc.version minimumRustVersion then
          pkgs
        else
          throw "Shadoword requires Rust ${minimumRustVersion} or newer; nixpkgs provides ${pkgs.rustc.version}";

      commonBuildDeps =
        pkgs: with pkgs; [
          cmake
          glslang
          gtk3
          libappindicator-gtk3
          llvmPackages.libclang
          makeWrapper
          pkg-config
          shaderc
          vulkan-headers
          vulkan-loader
          vulkan-tools
        ];

      daemonRuntimeDeps =
        pkgs: with pkgs; [
          stdenv.cc.cc.lib
          libglvnd
          libopus
          openssl
          vulkan-loader
        ];

      desktopRuntimeDeps =
        pkgs:
        daemonRuntimeDeps pkgs
        ++ (with pkgs; [
          alsa-lib
          fontconfig
          gtk3
          libappindicator-gtk3
          libevdev
          libopus
          libx11
          libxi
          libxtst
          libxcb
          libxkbcommon
          wayland
          xdotool
        ]);

      clientRuntimeDeps =
        pkgs: with pkgs; [
          stdenv.cc.cc.lib
          libglvnd
          vulkan-loader
          alsa-lib
          fontconfig
          glib-networking
          gtk3
          libappindicator-gtk3
          libevdev
          libopus
          libsoup_3
          libx11
          libxi
          libxtst
          libxcb
          libxkbcommon
          wayland
          webkitgtk_4_1
          xdotool
        ];

      tauriDesktopBuildDeps =
        pkgs: with pkgs; [
          bun
          libsoup_3
          webkitgtk_4_1
        ];

      commonEnv = pkgs: {
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.lib.getVersion pkgs.llvmPackages.libclang}/include -isystem ${pkgs.glibc.dev}/include";
        VULKAN_SDK = "${pkgs.vulkan-headers}";
      };

      cudaDeps =
        pkgs: with pkgs.cudaPackages; [
          cuda_cccl
          cuda_cudart
          cuda_nvcc
          libcublas
        ];

      cudaRuntimeDeps =
        pkgs:
        with pkgs.cudaPackages;
        map pkgs.lib.getLib [
          cuda_cudart
          libcublas
        ];

      cudaEnv =
        pkgs:
        let
          deps = cudaDeps pkgs;
          includePath = pkgs.lib.concatStringsSep ":" (map (pkg: "${pkg}/include") deps);
          libraryPath = pkgs.lib.makeLibraryPath deps;
        in
        commonEnv pkgs
        // {
          CUDA_HOME = "${pkgs.cudaPackages.cuda_nvcc}";
          CUDA_PATH = "${pkgs.cudaPackages.cuda_nvcc}";
          CUDACXX = "${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc";
          CMAKE_CUDA_COMPILER = "${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc";
          CUDAHOSTCXX = "${pkgs.gcc14}/bin/g++";
          CMAKE_CUDA_HOST_COMPILER = "${pkgs.gcc14}/bin/g++";
          CUDAARCHS = "86";
          CMAKE_CUDA_ARCHITECTURES = "86";
          CUDAToolkit_ROOT = "${pkgs.cudaPackages.cuda_nvcc}";
          CMAKE_PREFIX_PATH = "${pkgs.cudaPackages.cuda_nvcc}";
          CPATH = includePath;
          CPLUS_INCLUDE_PATH = includePath;
          LIBRARY_PATH = "${libraryPath}:${pkgs.cudaPackages.cuda_cudart}/lib/stubs";
        };

      runtimeLibraryPath =
        pkgs: runtimeDeps: "/run/opengl-driver/lib:${pkgs.lib.makeLibraryPath runtimeDeps}";

      mkContainerImage =
        {
          pkgs,
          apiPackage,
          variant,
        }:
        pkgs.dockerTools.buildLayeredImage {
          name = "shadoword-backend";
          tag = variant;
          contents = [
            apiPackage
            pkgs.cacert
          ]
          ++ pkgs.lib.optionals (variant == "vulkan") [ pkgs.mesa ];

          extraCommands = ''
            mkdir -p config data etc home/shadoword tmp usr/local/bin
            cat > etc/passwd <<'EOF'
            root:x:0:0:root:/root:/bin/sh
            shadoword:x:1000:1000:Shadoword:/home/shadoword:/bin/sh
            EOF
            cat > etc/group <<'EOF'
            root:x:0:
            shadoword:x:1000:
            EOF
            cat > etc/nsswitch.conf <<'EOF'
            passwd: files
            group: files
            hosts: files dns
            EOF
            printf '%s\n' '${variant}' > etc/shadoword-container-variant
            ln -s ${apiPackage}/bin/shadoword-api usr/local/bin/shadoword-api
            chmod 1777 tmp
            ${pkgs.lib.optionalString (variant == "vulkan") ''
              mkdir -p usr/share/vulkan
              ln -s ${pkgs.mesa}/share/vulkan/icd.d usr/share/vulkan/icd.d
            ''}
          '';
          fakeRootCommands = ''
            chown -R 1000:1000 config data home/shadoword
          '';

          config = {
            Entrypoint = [ "/usr/local/bin/shadoword-api" ];
            Env = [
              "HOME=/home/shadoword"
              "XDG_CONFIG_HOME=/config"
              "XDG_DATA_HOME=/data"
              "SHADOWORD_LISTEN_ADDR=0.0.0.0:47813"
              "SHADOWORD_CONTAINER_VARIANT=${variant}"
              "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
              "RUST_LOG=info"
            ];
            User = "1000:1000";
            WorkingDir = "/home/shadoword";
            ExposedPorts."47813/tcp" = { };
            # Hashed token records live in /config. Without a mount the daemon
            # loses every token it was given the moment the container is replaced.
            Volumes = {
              "/config" = { };
              "/data" = { };
            };
            Labels = {
              "io.shadoword.inference-variant" = variant;
              "org.opencontainers.image.description" = "Shadoword ${variant} speech-to-text API";
              "org.opencontainers.image.licenses" = "MIT";
              "org.opencontainers.image.revision" = self.rev or self.dirtyRev or "unknown";
              "org.opencontainers.image.source" = "https://github.com/Fractal-Tess/shadoword";
              "org.opencontainers.image.title" = "Shadoword API (${variant})";
              "org.opencontainers.image.version" = version;
            };
          };

          meta = {
            description = "Reproducible Shadoword ${variant} API container image";
            platforms = supportedSystems;
          };
        };

      mkRustPackage =
        {
          pkgs,
          system,
          pname,
          cargoPackage,
          executable ? cargoPackage,
          runtimeDeps,
          cargoFeatures ? [ ],
          cudaSupport ? false,
          noDefaultFeatures ? false,
          frontendNodeModules ? null,
          extraEnv ? { },
          desktopIntegration ? false,
        }:
        let
          packageRuntimeDeps = runtimeDeps ++ pkgs.lib.optionals cudaSupport (cudaRuntimeDeps pkgs);
          packageBuildDeps = packageRuntimeDeps ++ pkgs.lib.optionals cudaSupport (cudaDeps pkgs);
          desktopWrapperArgs = pkgs.lib.optionalString desktopIntegration "--set GDK_BACKEND x11 --set WEBKIT_DISABLE_DMABUF_RENDERER 1";
        in
        pkgs.rustPlatform.buildRustPackage {
          inherit pname version;
          src = self;

          cargoLock.lockFile = ./Cargo.lock;

          cargoBuildFlags = [
            "-p"
            cargoPackage
          ];
          buildFeatures = cargoFeatures;
          checkFeatures = cargoFeatures;
          buildNoDefaultFeatures = noDefaultFeatures;
          checkNoDefaultFeatures = noDefaultFeatures;

          nativeBuildInputs =
            commonBuildDeps pkgs
            ++ pkgs.lib.optionals cudaSupport [ pkgs.gcc14 ]
            ++ pkgs.lib.optionals (frontendNodeModules != null) [
              pkgs.bun
              pkgs.nodejs
            ];
          buildInputs = packageBuildDeps;

          env = (if cudaSupport then cudaEnv pkgs else commonEnv pkgs) // extraEnv;
          preBuild = pkgs.lib.optionalString (frontendNodeModules != null) ''
            unset LIBOPUS_STATIC OPUS_STATIC
            cp -R ${frontendNodeModules} crates/shadoword-desktop/node_modules
            chmod -R u+w crates/shadoword-desktop/node_modules
            patchShebangs crates/shadoword-desktop/node_modules
            (cd crates/shadoword-desktop && bun run build)
          '';

          doCheck = false;

          postInstall = ''
            strip --strip-unneeded "$out/bin/${executable}"
            wrapProgram "$out/bin/${executable}" \
              --prefix LD_LIBRARY_PATH : "${runtimeLibraryPath pkgs packageRuntimeDeps}" \
              ${desktopWrapperArgs}
            ${pkgs.lib.optionalString desktopIntegration ''
              install -Dm644 ${./nix/shadoword.desktop} "$out/share/applications/shadoword.desktop"
              install -Dm644 ${./crates/shadoword-desktop/src-tauri/icons/128x128.png} \
                "$out/share/icons/hicolor/128x128/apps/shadoword.png"
            ''}
          '';

          meta = {
            description = "Linux-first speech-to-text, local by default, with Shadoword API and OpenRouter options";
            homepage = "https://github.com/Fractal-Tess/shadoword";
            license = pkgs.lib.licenses.mit;
            mainProgram = executable;
            platforms = supportedSystems;
          };
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          desktopNodeModules = pkgs.stdenvNoCC.mkDerivation {
            pname = "shadoword-desktop-node-modules";
            inherit version;
            src = self;

            impureEnvVars = pkgs.lib.fetchers.proxyImpureEnvVars ++ [
              "GIT_PROXY_COMMAND"
              "SOCKS_SERVER"
            ];
            nativeBuildInputs = [
              pkgs.bun
              pkgs.writableTmpDirAsHomeHook
            ];
            dontUnpack = true;
            dontConfigure = true;
            dontFixup = true;

            buildPhase = ''
              runHook preBuild
              cp -R "$src/crates/shadoword-desktop" desktop
              chmod -R u+w desktop
              cd desktop
              export BUN_INSTALL_CACHE_DIR=$(mktemp -d)
              bun install --frozen-lockfile --ignore-scripts --no-progress
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              cp -R node_modules "$out"
              runHook postInstall
            '';

            outputHash = "sha256-LMB+D2FrUTkBdKnh27/bjELxNE166BpEs3t3YZOlrS0=";
            outputHashAlgo = "sha256";
            outputHashMode = "recursive";
          };
          sourcePackages = rec {
            shadoword-api = mkRustPackage {
              inherit pkgs system;
              pname = "shadoword-api";
              cargoPackage = "shadoword-api";
              runtimeDeps = daemonRuntimeDeps pkgs;
            };

            shadoword-api-cuda = mkRustPackage {
              inherit pkgs system;
              pname = "shadoword-api-cuda";
              cargoPackage = "shadoword-api";
              runtimeDeps = daemonRuntimeDeps pkgs;
              cargoFeatures = [ "whisper-cuda" ];
              cudaSupport = true;
            };

            shadoword-api-vulkan = mkRustPackage {
              inherit pkgs system;
              pname = "shadoword-api-vulkan";
              cargoPackage = "shadoword-api";
              runtimeDeps = daemonRuntimeDeps pkgs;
              cargoFeatures = [ "whisper-vulkan" ];
            };

            shadoword-desktop = mkRustPackage {
              inherit pkgs system;
              pname = "shadoword-desktop";
              cargoPackage = "shadoword-desktop";
              executable = "shadoword";
              runtimeDeps = clientRuntimeDeps pkgs;
              cargoFeatures = [
                "local-runtime"
                "custom-protocol"
              ];
              noDefaultFeatures = true;
              frontendNodeModules = desktopNodeModules;
              extraEnv.GGML_NATIVE = "OFF";
              desktopIntegration = true;
            };

            shadoword-desktop-cuda = mkRustPackage {
              inherit pkgs system;
              pname = "shadoword-desktop-cuda";
              cargoPackage = "shadoword-desktop";
              executable = "shadoword";
              runtimeDeps = clientRuntimeDeps pkgs;
              cargoFeatures = [
                "local-runtime"
                "custom-protocol"
                "whisper-cuda"
              ];
              cudaSupport = true;
              noDefaultFeatures = true;
              frontendNodeModules = desktopNodeModules;
              extraEnv.GGML_NATIVE = "OFF";
              desktopIntegration = true;
            };

            shadoword-desktop-vulkan = mkRustPackage {
              inherit pkgs system;
              pname = "shadoword-desktop-vulkan";
              cargoPackage = "shadoword-desktop";
              executable = "shadoword";
              runtimeDeps = clientRuntimeDeps pkgs;
              cargoFeatures = [
                "local-runtime"
                "custom-protocol"
                "whisper-vulkan"
              ];
              noDefaultFeatures = true;
              frontendNodeModules = desktopNodeModules;
              extraEnv.GGML_NATIVE = "OFF";
              desktopIntegration = true;
            };

            # Keep the historical package name as an alias to the unified desktop build.
            shadoword-desktop-client = shadoword-desktop;
          };
          packageFor =
            packageName:
            {
              executable,
              runtimeDeps,
              extraLibraryPath ? "",
              desktopIntegration ? false,
            }:
            let
              artifact = artifactFor system packageName;
            in
            if artifact == null then
              sourcePackages.${packageName}
            else
              pkgs.callPackage ./nix/prebuilt-package.nix {
                inherit
                  artifact
                  executable
                  extraLibraryPath
                  runtimeDeps
                  version
                  desktopIntegration
                  ;
                pname = packageName;
                desktopFile = ./nix/shadoword.desktop;
                desktopIcon = ./crates/shadoword-desktop/src-tauri/icons/128x128.png;
              };
        in
        rec {
          default = shadoword-api;

          shadoword-api = packageFor "shadoword-api" {
            executable = "shadoword-api";
            runtimeDeps = daemonRuntimeDeps pkgs;
          };
          shadoword-api-cuda = packageFor "shadoword-api-cuda" {
            executable = "shadoword-api";
            runtimeDeps = daemonRuntimeDeps pkgs ++ cudaRuntimeDeps pkgs;
            extraLibraryPath = "/run/opengl-driver/lib";
          };
          shadoword-api-vulkan = packageFor "shadoword-api-vulkan" {
            executable = "shadoword-api";
            runtimeDeps = daemonRuntimeDeps pkgs;
            extraLibraryPath = "/run/opengl-driver/lib";
          };
          shadoword-desktop = packageFor "shadoword-desktop" {
            executable = "shadoword";
            runtimeDeps = clientRuntimeDeps pkgs;
            extraLibraryPath = "/run/opengl-driver/lib";
            desktopIntegration = true;
          };
          shadoword-desktop-cuda = packageFor "shadoword-desktop-cuda" {
            executable = "shadoword";
            runtimeDeps = clientRuntimeDeps pkgs ++ cudaRuntimeDeps pkgs;
            extraLibraryPath = "/run/opengl-driver/lib";
            desktopIntegration = true;
          };
          shadoword-desktop-vulkan = packageFor "shadoword-desktop-vulkan" {
            executable = "shadoword";
            runtimeDeps = clientRuntimeDeps pkgs;
            extraLibraryPath = "/run/opengl-driver/lib";
            desktopIntegration = true;
          };
          # Downstream configurations can keep using the old name without losing local inference.
          shadoword-desktop-client = shadoword-desktop;

          shadoword-api-source = sourcePackages.shadoword-api;
          shadoword-api-cuda-source = sourcePackages.shadoword-api-cuda;
          shadoword-api-vulkan-source = sourcePackages.shadoword-api-vulkan;
          shadoword-container-cpu = mkContainerImage {
            inherit pkgs;
            apiPackage = sourcePackages.shadoword-api;
            variant = "cpu";
          };
          shadoword-container-cuda = mkContainerImage {
            inherit pkgs;
            apiPackage = sourcePackages.shadoword-api-cuda;
            variant = "cuda";
          };
          shadoword-container-vulkan = mkContainerImage {
            inherit pkgs;
            apiPackage = sourcePackages.shadoword-api-vulkan;
            variant = "vulkan";
          };
          shadoword-desktop-source = sourcePackages.shadoword-desktop;
          shadoword-desktop-cuda-source = sourcePackages.shadoword-desktop-cuda;
          shadoword-desktop-vulkan-source = sourcePackages.shadoword-desktop-vulkan;
          shadoword-desktop-client-source = shadoword-desktop-source;
        }
      );

      nixosModules = rec {
        shadoword-api = import ./nix/nixos-module.nix self;
        default = shadoword-api;
      };

      # The packages are taken from this flake's own nixpkgs rather than rebuilt
      # against the consumer's, so that pulling in the CUDA daemon does not make
      # the host's nixpkgs config carry `allowUnfree`.
      overlays.default =
        _final: prev:
        let
          flakePackages = self.packages.${prev.stdenv.hostPlatform.system};
        in
        {
          inherit (flakePackages)
            shadoword-api
            shadoword-api-cuda
            shadoword-api-vulkan
            shadoword-desktop
            shadoword-desktop-cuda
            shadoword-desktop-vulkan
            ;
        };

      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          runtimeDeps = desktopRuntimeDeps pkgs;
          cudaPkgs = cudaDeps pkgs;
          cudaIncludePath = pkgs.lib.concatStringsSep ":" (map (pkg: "${pkg}/include") cudaPkgs);
        in
        {
          default = pkgs.mkShell {
            buildInputs =
              commonBuildDeps pkgs
              ++ runtimeDeps
              ++ tauriDesktopBuildDeps pkgs
              ++ (with pkgs; [
                cargo
                clippy
                rust-analyzer
                rustc
                rustfmt
              ]);

            inherit (commonEnv pkgs)
              LIBCLANG_PATH
              BINDGEN_EXTRA_CLANG_ARGS
              VULKAN_SDK
              ;

            LD_LIBRARY_PATH = runtimeLibraryPath pkgs runtimeDeps;

            shellHook = ''
              export VK_DRIVER_FILES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
              export VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
              export VK_LAYER_PATH=/run/opengl-driver/share/vulkan/implicit_layer.d:/run/opengl-driver/share/vulkan/explicit_layer.d
              if [ -n "''${XDG_DATA_DIRS:-}" ]; then
                export XDG_DATA_DIRS=/run/opengl-driver/share:$XDG_DATA_DIRS
              else
                export XDG_DATA_DIRS=/run/opengl-driver/share
              fi
              echo "Shadoword Whisper development environment"
              echo "Run 'cd crates/shadoword-desktop && bun run tauri dev -- --features whisper-vulkan' for the desktop"
              echo "For CUDA, enter 'nix develop .#cuda' then run 'cd crates/shadoword-desktop && bun run tauri dev -- --features whisper-cuda'"
              echo "Run 'cargo run -p shadoword-api --features whisper-vulkan' for daemon Vulkan"
            '';
          };

          cuda = pkgs.mkShell {
            buildInputs =
              commonBuildDeps pkgs
              ++ runtimeDeps
              ++ tauriDesktopBuildDeps pkgs
              ++ (with pkgs; [
                cargo
                clippy
                rust-analyzer
                rustc
                rustfmt
              ])
              ++ cudaPkgs;

            inherit (commonEnv pkgs)
              LIBCLANG_PATH
              BINDGEN_EXTRA_CLANG_ARGS
              VULKAN_SDK
              ;

            LD_LIBRARY_PATH = runtimeLibraryPath pkgs runtimeDeps;

            CUDA_PATH = "${pkgs.cudaPackages.cuda_nvcc}";

            shellHook = ''
              export VK_DRIVER_FILES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
              export VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
              export VK_LAYER_PATH=/run/opengl-driver/share/vulkan/implicit_layer.d:/run/opengl-driver/share/vulkan/explicit_layer.d
              if [ -n "''${XDG_DATA_DIRS:-}" ]; then
                export XDG_DATA_DIRS=/run/opengl-driver/share:$XDG_DATA_DIRS
              else
                export XDG_DATA_DIRS=/run/opengl-driver/share
              fi

              # Use a single CUDA toolchain from nixpkgs for compile/link/runtime.
              # Avoid mixing with ~/.local/cuda-toolkit wrappers, which causes
              # inconsistent header/lib discovery in CMake + whisper-rs-sys.
              export CUDA_HOME="${pkgs.cudaPackages.cuda_nvcc}"
              export CUDA_PATH="$CUDA_HOME"
              export PATH="$CUDA_HOME/bin:$PATH"
              export CUDACXX="$CUDA_HOME/bin/nvcc"
              export CMAKE_CUDA_COMPILER="$CUDACXX"

              # CUDA 12.9 supports GCC 14 but not the current nixpkgs GCC 15.
              # CMake's CUDA compiler-id test otherwise fails before Rust builds.
              export CUDAHOSTCXX="${pkgs.gcc14}/bin/g++"
              export CMAKE_CUDA_HOST_COMPILER="$CUDAHOSTCXX"
              export CUDAARCHS="86"
              export CMAKE_CUDA_ARCHITECTURES="86"

              # Extra hints for CMake projects that use CUDAToolkit_ROOT.
              export CUDAToolkit_ROOT="$CUDA_HOME"
              export CMAKE_PREFIX_PATH="$CUDA_HOME:''${CMAKE_PREFIX_PATH:-}"
              export CPATH=${cudaIncludePath}:''${CPATH:-}
              export CPLUS_INCLUDE_PATH=${cudaIncludePath}:''${CPLUS_INCLUDE_PATH:-}
              unset CUDA_INC_PATH

              # Make CUDA libs available for both linker-time and runtime.
              export LIBRARY_PATH=/run/opengl-driver/lib:${pkgs.lib.makeLibraryPath cudaPkgs}:$LIBRARY_PATH
              export LD_LIBRARY_PATH=/run/opengl-driver/lib:${pkgs.lib.makeLibraryPath cudaPkgs}:$LD_LIBRARY_PATH

              echo "Shadoword development environment (CUDA)"
              echo "CUDA toolkit: $CUDA_HOME"
              echo "CUDA compiler: $CUDACXX"
              echo "CUDA host compiler: $CUDAHOSTCXX"
              echo "Run 'cd crates/shadoword-desktop && bun run tauri dev -- --features whisper-cuda' for the desktop"
              echo "Run 'cargo run -p shadoword-api --features whisper-cuda' for daemon CUDA"
            '';
          };
        }
      );
    };
}
