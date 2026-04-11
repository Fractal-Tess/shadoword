{
  description = "Shadow Word - A free, open source, and extensible speech-to-text application that works completely offline";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # bun2nix: generates per-package Nix fetchurl expressions from bun.lock,
    # replacing the old FOD approach where a single hash covered the entire
    # node_modules directory (that hash would break on bun version changes).
    # See: https://github.com/nix-community/bun2nix
    bun2nix = {
      url = "github:nix-community/bun2nix/2.0.8";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      bun2nix,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      # Read version from Cargo.toml
      cargoToml = fromTOML (builtins.readFile ./src-tauri/Cargo.toml);
      version = cargoToml.package.version;
      mkPkgs =
        {
          system,
          cuda ? false,
        }:
        import nixpkgs {
          inherit system;
          overlays = [
            bun2nix.overlays.default
          ];
          config = nixpkgs.lib.optionalAttrs cuda {
            allowUnfree = true;
            cudaSupport = true;
            rocmSupport = false;
            cudaCapabilities = [ "8.6" ];
            cudaForwardCompat = false;
          };
        };

      cudaRuntimeDeps = pkgs:
        with pkgs.cudaPackages;
        [
          cuda_cudart
          libcublas
          libcurand
          libcusparse
          libcufft
          cuda_nvrtc
          cudnn
        ];

      mkPrebuiltOnnxruntimeGpu =
        pkgs:
        pkgs.stdenvNoCC.mkDerivation rec {
          pname = "onnxruntime-gpu-prebuilt";
          version = "1.24.2";

          src = pkgs.fetchurl {
            url = "https://github.com/microsoft/onnxruntime/releases/download/v${version}/onnxruntime-linux-x64-gpu-${version}.tgz";
            hash = "sha256-vLQtoEH0IZLlV53hdfdBAxPBFHQKYR4jCv6deb5lzEk=";
          };

          dontConfigure = true;
          dontBuild = true;

          installPhase = ''
            runHook preInstall
            mkdir -p "$out"
            tar xzf "$src" --strip-components=1 -C "$out"
            runHook postInstall
          '';
        };

      # Shared native library dependencies for both package build and dev shell.
      # Keep in sync: if a native dep is needed for compilation, add it here.
      commonNativeDeps = pkgs: onnxruntimePkg: with pkgs; [
        webkitgtk_4_1
        gtk3
        glib
        libsoup_3
        alsa-lib
        onnxruntimePkg
        libayatana-appindicator
        libevdev
        libxtst
        gtk-layer-shell
        openssl
        vulkan-loader
        vulkan-headers
        shaderc
      ];

      # GStreamer plugins for WebKitGTK audio/video
      gstPlugins = pkgs: with pkgs.gst_all_1; [
        gstreamer
        gst-plugins-base
        gst-plugins-good
        gst-plugins-bad
        gst-plugins-ugly
      ];

      # Shared environment variables for Rust/native builds
      commonEnv = pkgs: onnxruntimePkg: let lib = pkgs.lib; in {
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${lib.getVersion pkgs.llvmPackages.libclang}/include -isystem ${pkgs.glibc.dev}/include";
        ORT_LIB_LOCATION = "${onnxruntimePkg}/lib";
        ORT_PREFER_DYNAMIC_LINK = "1";
        GST_PLUGIN_SYSTEM_PATH_1_0 = "${lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" (gstPlugins pkgs)}";
      };

      mkShadowwordPackage =
        {
          pkgs,
          pname ? "shadowword",
          onnxruntimePkg ? pkgs.onnxruntime,
          cargoFeatures ? [ ],
          extraRuntimeLibs ? [ ],
        }:
        let
          lib = pkgs.lib;
          combinedAlsaPlugins = pkgs.symlinkJoin {
            name = "combined-alsa-plugins";
            paths = [
              "${pkgs.pipewire}/lib/alsa-lib"
              "${pkgs.alsa-plugins}/lib/alsa-lib"
            ];
          };
        in
        pkgs.rustPlatform.buildRustPackage {
          inherit pname version;
          src = self;

          cargoRoot = "src-tauri";
          cargoBuildFlags = lib.optionals (cargoFeatures != [ ]) [
            "--features"
            (lib.concatStringsSep "," cargoFeatures)
          ];

          cargoLock = {
            lockFile = ./src-tauri/Cargo.lock;
            # Automatically fetch git dependencies using builtins.fetchGit.
            # This eliminates the need for manual outputHashes that had to be
            # updated every time a git dependency changed in Cargo.lock.
            # Safe for standalone flakes (not allowed in nixpkgs, it is needed something like crate2nix).
            allowBuiltinFetchGit = true;
          };

          postPatch = ''
            ${pkgs.jq}/bin/jq 'del(.build.beforeBuildCommand) | .bundle.createUpdaterArtifacts = false' \
              src-tauri/tauri.conf.json > $TMPDIR/tauri.conf.json
            cp $TMPDIR/tauri.conf.json src-tauri/tauri.conf.json

            # Strip postinstall hook — it runs check-nix-deps.ts which is only
            # needed during local development, not inside the Nix sandbox.
            ${pkgs.jq}/bin/jq 'del(.scripts.postinstall)' \
              package.json > $TMPDIR/package.json
            cp $TMPDIR/package.json package.json

            # Point libappindicator-sys to the Nix store path
            substituteInPlace \
              $cargoDepsCopy/libappindicator-sys-*/src/lib.rs \
              --replace-fail \
                "libayatana-appindicator3.so.1" \
                "${pkgs.libayatana-appindicator}/lib/libayatana-appindicator3.so.1"

            # Disable cbindgen in ferrous-opencc (calls cargo metadata which fails in sandbox)
            # Upstream removed this call in v0.3.1+
            substituteInPlace $cargoDepsCopy/ferrous-opencc-0.2.3/build.rs \
              --replace-fail '.expect("Unable to generate bindings")' '.ok();'
            substituteInPlace $cargoDepsCopy/ferrous-opencc-0.2.3/build.rs \
              --replace-fail '.write_to_file("opencc.h");' '// skipped'
          '';

          # Bun dependencies: fetched per-package using hashes from .nix/bun.nix.
          # This file is auto-generated by `bunx bun2nix -o .nix/bun.nix` and
          # kept in sync via the postinstall hook in package.json.
          # To regenerate manually: bun scripts/check-nix-deps.ts
          bunDeps = pkgs.bun2nix.fetchBunDeps {
            bunNix = ./.nix/bun.nix;
          };

          nativeBuildInputs = with pkgs; [
            cargo-tauri.hook
            pkg-config
            wrapGAppsHook4
            bun
            # pkgs.bun2nix (from overlay), not the flake input — `with pkgs;`
            # doesn't shadow function arguments in Nix.
            pkgs.bun2nix.hook # Sets up node_modules from pre-fetched bun cache
            jq
            cmake
            llvmPackages.libclang
            shaderc
          ];

          preBuild = ''
            # bun2nix.hook has already set up node_modules from pre-fetched cache.
            # Build the frontend with bun (tsc + vite).
            export HOME=$TMPDIR
            bun run build
          '';

          # Tests require runtime resources (audio devices, model files, GPU/Vulkan)
          # not available in the Nix build sandbox
          doCheck = false;

          # The tauri hook's installPhase expects target/ in cwd, but our
          # cargoRoot puts it under src-tauri/. Override to extract the DEB.
          installPhase = ''
            runHook preInstall
            mkdir -p $out
            cd src-tauri
            mv target/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/release/bundle/deb/*/data/usr/* $out/
            runHook postInstall
          '';

          buildInputs = commonNativeDeps pkgs onnxruntimePkg ++ (with pkgs; [
            glib-networking
            libx11
          ]) ++ extraRuntimeLibs ++ gstPlugins pkgs;

          env = commonEnv pkgs onnxruntimePkg // {
            OPENSSL_NO_VENDOR = "1";
          };

          preFixup = ''
            gappsWrapperArgs+=(
              --set WEBKIT_DISABLE_DMABUF_RENDERER 1
              --set ALSA_PLUGIN_DIR "${combinedAlsaPlugins}"
              --prefix LD_LIBRARY_PATH : "${
                lib.makeLibraryPath (
                  [
                    pkgs.vulkan-loader
                    onnxruntimePkg
                  ]
                  ++ extraRuntimeLibs
                )
              }"
            )
          '';

          meta = {
            description = "A free, open source, and extensible speech-to-text application that works completely offline";
            homepage = "https://github.com/Fractal-Tess/shadow-word";
            license = lib.licenses.mit;
            mainProgram = "shadowword";
            platforms = supportedSystems;
          };
        };

    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = mkPkgs {
            inherit system;
          };
          cudaPkgs = mkPkgs {
            inherit system;
            cuda = system == "x86_64-linux";
          };
          prebuiltOnnxruntimeGpu = mkPrebuiltOnnxruntimeGpu cudaPkgs;
        in
        {
          shadowword = mkShadowwordPackage {
            inherit pkgs;
          };

          shadowword-cuda = mkShadowwordPackage {
            pkgs = cudaPkgs;
            pname = "shadowword-cuda";
            onnxruntimePkg = prebuiltOnnxruntimeGpu;
            cargoFeatures = [ "linux-cuda" ];
            extraRuntimeLibs = nixpkgs.lib.optionals (system == "x86_64-linux") (cudaRuntimeDeps cudaPkgs);
          };

          default = self.packages.${system}.shadowword;
        }
      );

      # NixOS module for system-level integration (udev, input group)
      nixosModules.default =
        { lib, pkgs, ... }:
        {
          imports = [ ./nix/module.nix ];
          programs.shadowword.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.shadowword;
        };

      # Home-manager module for per-user service
      homeManagerModules.default =
        { lib, pkgs, ... }:
        {
          imports = [ ./nix/hm-module.nix ];
          services.shadowword.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.shadowword;
        };

      # Development shell for building from source
      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs {
            inherit system;
          };
          cudaPkgs = mkPkgs {
            inherit system;
            cuda = system == "x86_64-linux";
          };
          prebuiltOnnxruntimeGpu = mkPrebuiltOnnxruntimeGpu cudaPkgs;
        in
        {
          default = pkgs.mkShell {
            buildInputs = commonNativeDeps pkgs pkgs.onnxruntime ++ (with pkgs; [
              # Rust toolchain
              rustc
              cargo
              rust-analyzer
              clippy
              # Frontend
              nodejs
              bun
              # Build tools
              cargo-tauri
              pkg-config
              llvmPackages.libclang
              cmake
            ]);

            inherit (commonEnv pkgs pkgs.onnxruntime)
              LIBCLANG_PATH
              BINDGEN_EXTRA_CLANG_ARGS
              ORT_LIB_LOCATION
              ORT_PREFER_DYNAMIC_LINK
              GST_PLUGIN_SYSTEM_PATH_1_0;

            LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [ pkgs.libayatana-appindicator pkgs.onnxruntime pkgs.vulkan-loader ]}";

            # Same as wrapGAppsHook4
            XDG_DATA_DIRS = "${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:${pkgs.hicolor-icon-theme}/share";

            shellHook = ''
              echo "Shadow Word development environment"
              bun install
              echo "Run 'bun run tauri dev' to start"
            '';
          };

          cuda = cudaPkgs.mkShell {
            buildInputs =
              commonNativeDeps cudaPkgs prebuiltOnnxruntimeGpu
              ++ (with cudaPkgs; [
                rustc
                cargo
                rust-analyzer
                clippy
                nodejs
                bun
                cargo-tauri
                pkg-config
                llvmPackages.libclang
                cmake
              ])
              ++ cudaRuntimeDeps cudaPkgs;

            inherit (commonEnv cudaPkgs prebuiltOnnxruntimeGpu)
              LIBCLANG_PATH
              BINDGEN_EXTRA_CLANG_ARGS
              ORT_LIB_LOCATION
              ORT_PREFER_DYNAMIC_LINK
              GST_PLUGIN_SYSTEM_PATH_1_0;

            LD_LIBRARY_PATH = "${cudaPkgs.lib.makeLibraryPath ([ cudaPkgs.libayatana-appindicator prebuiltOnnxruntimeGpu cudaPkgs.vulkan-loader ] ++ cudaRuntimeDeps cudaPkgs)}";

            XDG_DATA_DIRS = "${cudaPkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${cudaPkgs.gsettings-desktop-schemas.name}:${cudaPkgs.gtk3}/share/gsettings-schemas/${cudaPkgs.gtk3.name}:${cudaPkgs.hicolor-icon-theme}/share";

            shellHook = ''
              echo "Shadow Word CUDA development environment"
              bun install
              echo "Run 'cargo build --features linux-cuda' or 'bun run tauri dev'"
            '';
          };
        }
      );
    };
}
