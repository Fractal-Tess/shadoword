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
          gtk3
          libappindicator-gtk3
          llvmPackages.libclang
          makeWrapper
          pkg-config
          shaderc
          vulkan-headers
          vulkan-loader
          vulkan-tools
          xdotool
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
          gtk3
          libappindicator-gtk3
          libevdev
          libx11
          libxi
          libxtst
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
            cargoPackage = "shadoword-egui";
            runtimeDeps = desktopRuntimeDeps pkgs;
          };

          shadoword = mkRustPackage {
            inherit pkgs;
            inherit system;
            pname = "shadoword";
            cargoPackage = "shadoword-egui";
            runtimeDeps = desktopRuntimeDeps pkgs;
          };

          shadoword-egui = mkRustPackage {
            inherit pkgs;
            inherit system;
            pname = "shadoword-egui";
            cargoPackage = "shadoword-egui";
            runtimeDeps = desktopRuntimeDeps pkgs;
          };

          shadoword-api = mkRustPackage {
            inherit pkgs;
            inherit system;
            pname = "shadoword-api";
            cargoPackage = "shadoword-api";
            runtimeDeps = daemonRuntimeDeps pkgs;
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          runtimeDeps = desktopRuntimeDeps pkgs;
          cudaPkgs = with pkgs.cudaPackages; [
            cudnn  # cuDNN 9.x for CUDA 12.9
            libcublas
            libcufft
            libcurand
            cuda_nvrtc
          ];
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
              export VK_DRIVER_FILES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
              export VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
              export VK_LAYER_PATH=/run/opengl-driver/share/vulkan/implicit_layer.d:/run/opengl-driver/share/vulkan/explicit_layer.d
              if [ -n "''${XDG_DATA_DIRS:-}" ]; then
                export XDG_DATA_DIRS=/run/opengl-driver/share:$XDG_DATA_DIRS
              else
                export XDG_DATA_DIRS=/run/opengl-driver/share
              fi
              echo "Shadoword Whisper development environment"
              echo "Run 'cargo run -p shadoword-egui --features whisper-vulkan' for Vulkan"
              echo "Run 'cargo run -p shadoword-egui --features whisper-cuda' for CUDA"
              echo "Run 'cargo run -p shadoword-api --features whisper-vulkan' for daemon Vulkan"
            '';
          };

          cuda = pkgs.mkShell {
            buildInputs =
              commonBuildDeps pkgs
              ++ runtimeDeps
              ++ (with pkgs; [
                cargo
                clippy
                rust-analyzer
                rustc
              ])
              ++ (with pkgs.cudaPackages; [
                cuda_cudart       # CUDA runtime (libcudart)
                cudnn             # cuDNN 9.x (runtime libs)
                libcublas
                libcufft
                libcurand
                cuda_nvrtc
              ]);

            inherit (commonEnv pkgs)
              LIBCLANG_PATH
              BINDGEN_EXTRA_CLANG_ARGS
              VULKAN_SDK;

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
              export CUDACXX="${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc"
              export CMAKE_CUDA_COMPILER="$CUDACXX"

              # Extra hints for CMake projects that use CUDAToolkit_ROOT.
              export CUDAToolkit_ROOT="$CUDA_HOME"

              # Make CUDA libs available for both linker-time and runtime.
              export LIBRARY_PATH=/run/opengl-driver/lib:${pkgs.lib.makeLibraryPath (with pkgs.cudaPackages; [ cuda_cudart libcublas libcufft libcurand cuda_nvrtc cudnn ])}:$LIBRARY_PATH
              export LD_LIBRARY_PATH=/run/opengl-driver/lib:${pkgs.lib.makeLibraryPath (with pkgs.cudaPackages; [ cuda_cudart libcublas libcufft libcurand cuda_nvrtc cudnn ])}:$LD_LIBRARY_PATH

              echo "Shadoword development environment (CUDA)"
              echo "CUDA toolkit: $CUDA_HOME"
              echo "CUDA compiler: $CUDACXX"
              echo "Run with: cargo test --workspace"
            '';
          };
        }
      );
    };
}
