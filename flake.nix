{
  description = "Shadow Word - Rust workspace for the egui desktop client and remote daemon";

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

      mkPkgs = { system, cuda ? false }:
        import nixpkgs {
          inherit system;
          config = nixpkgs.lib.optionalAttrs cuda {
            allowUnfree = true;
            cudaSupport = true;
            rocmSupport = false;
            cudaCapabilities = [ "8.6" ];
            cudaForwardCompat = false;
          };
        };

      cudaRuntimeDeps = pkgs:
        with pkgs.cudaPackages; [
          cuda_cudart
          libcublas
          libcurand
          libcusparse
          libcufft
          cuda_nvrtc
          cudnn
        ];

      mkPrebuiltOnnxruntimeGpu = pkgs:
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

      commonRuntimeDeps = pkgs: onnxruntimePkg:
        with pkgs; [
          alsa-lib
          fontconfig
          libglvnd
          libx11
          libxcb
          libxkbcommon
          onnxruntimePkg
          openssl
          vulkan-loader
          wayland
        ];

      commonBuildDeps = pkgs:
        with pkgs; [
          cmake
          llvmPackages.libclang
          makeWrapper
          pkg-config
        ];

      commonEnv = pkgs: onnxruntimePkg: {
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.lib.getVersion pkgs.llvmPackages.libclang}/include -isystem ${pkgs.glibc.dev}/include";
        ORT_LIB_LOCATION = "${onnxruntimePkg}/lib";
        ORT_PREFER_DYNAMIC_LINK = "1";
      };

      runtimeLibraryPath = pkgs: onnxruntimePkg: extraLibs:
        "/run/opengl-driver/lib:${pkgs.lib.makeLibraryPath (commonRuntimeDeps pkgs onnxruntimePkg ++ extraLibs)}";

      mkRustPackage =
        {
          pkgs,
          pname,
          cargoPackage,
          onnxruntimePkg ? pkgs.onnxruntime,
          cargoFeatures ? [ ],
          extraRuntimeLibs ? [ ],
        }:
        pkgs.rustPlatform.buildRustPackage {
          inherit pname version;
          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          cargoBuildFlags =
            [
              "-p"
              cargoPackage
            ]
            ++ pkgs.lib.optionals (cargoFeatures != [ ]) [
              "--features"
              (pkgs.lib.concatStringsSep "," cargoFeatures)
            ];

          nativeBuildInputs = commonBuildDeps pkgs;
          buildInputs = commonRuntimeDeps pkgs onnxruntimePkg ++ extraRuntimeLibs;

          env = commonEnv pkgs onnxruntimePkg;
          doCheck = false;

          postInstall = ''
            wrapProgram "$out/bin/${cargoPackage}" \
              --prefix LD_LIBRARY_PATH : "${runtimeLibraryPath pkgs onnxruntimePkg extraRuntimeLibs}"
          '';

          meta = {
            description = "Offline speech-to-text workspace with egui desktop UI and optional daemon";
            homepage = "https://github.com/Fractal-Tess/shadoword";
            license = pkgs.lib.licenses.mit;
            platforms = supportedSystems;
          };
        };
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = mkPkgs { inherit system; };
          cudaPkgs = mkPkgs { inherit system; cuda = system == "x86_64-linux"; };
          prebuiltOnnxruntimeGpu = mkPrebuiltOnnxruntimeGpu cudaPkgs;
        in
        {
          default = mkRustPackage {
            inherit pkgs;
            pname = "shadowword-desktop";
            cargoPackage = "shadowword-desktop";
          };

          shadowword-desktop = mkRustPackage {
            inherit pkgs;
            pname = "shadowword-desktop";
            cargoPackage = "shadowword-desktop";
          };

          shadowword-daemon = mkRustPackage {
            inherit pkgs;
            pname = "shadowword-daemon";
            cargoPackage = "shadowword-daemon";
          };
        }
        // pkgs.lib.optionalAttrs (system == "x86_64-linux") {
          shadowword-desktop-cuda = mkRustPackage {
            pkgs = cudaPkgs;
            pname = "shadowword-desktop-cuda";
            cargoPackage = "shadowword-desktop";
            onnxruntimePkg = prebuiltOnnxruntimeGpu;
            cargoFeatures = [ "cuda" ];
            extraRuntimeLibs = cudaRuntimeDeps cudaPkgs;
          };

          shadowword-daemon-cuda = mkRustPackage {
            pkgs = cudaPkgs;
            pname = "shadowword-daemon-cuda";
            cargoPackage = "shadowword-daemon";
            onnxruntimePkg = prebuiltOnnxruntimeGpu;
            cargoFeatures = [ "cuda" ];
            extraRuntimeLibs = cudaRuntimeDeps cudaPkgs;
          };
        });

      devShells = forAllSystems (system:
        let
          pkgs = mkPkgs { inherit system; };
          cudaPkgs = mkPkgs { inherit system; cuda = system == "x86_64-linux"; };
          prebuiltOnnxruntimeGpu = mkPrebuiltOnnxruntimeGpu cudaPkgs;
        in
        {
          default = pkgs.mkShell {
            buildInputs =
              commonBuildDeps pkgs
              ++ commonRuntimeDeps pkgs pkgs.onnxruntime
              ++ (with pkgs; [
                cargo
                clippy
                rust-analyzer
                rustc
              ]);

            inherit (commonEnv pkgs pkgs.onnxruntime)
              LIBCLANG_PATH
              BINDGEN_EXTRA_CLANG_ARGS
              ORT_LIB_LOCATION
              ORT_PREFER_DYNAMIC_LINK;

            LD_LIBRARY_PATH = runtimeLibraryPath pkgs pkgs.onnxruntime [ ];

            shellHook = ''
              echo "Shadow Word Rust development environment"
              echo "Run 'cargo run -p shadowword-desktop' for the egui app"
              echo "Run 'cargo run -p shadowword-daemon' for the remote daemon"
            '';
          };

          cuda = cudaPkgs.mkShell {
            buildInputs =
              commonBuildDeps cudaPkgs
              ++ commonRuntimeDeps cudaPkgs prebuiltOnnxruntimeGpu
              ++ cudaRuntimeDeps cudaPkgs
              ++ (with cudaPkgs; [
                cargo
                clippy
                rust-analyzer
                rustc
              ]);

            inherit (commonEnv cudaPkgs prebuiltOnnxruntimeGpu)
              LIBCLANG_PATH
              BINDGEN_EXTRA_CLANG_ARGS
              ORT_LIB_LOCATION
              ORT_PREFER_DYNAMIC_LINK;

            LD_LIBRARY_PATH = runtimeLibraryPath cudaPkgs prebuiltOnnxruntimeGpu (cudaRuntimeDeps cudaPkgs);

            shellHook = ''
              echo "Shadow Word CUDA development environment"
              echo "Run 'cargo run -p shadowword-desktop --features cuda' for the egui app"
              echo "Run 'cargo run -p shadowword-daemon --features cuda' for the remote daemon"
            '';
          };
        });
    };
}
