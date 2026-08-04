{
  lib,
  stdenvNoCC,
  fetchurl,
  autoPatchelfHook,
  makeWrapper,
  version,
  pname,
  executable,
  artifact,
  runtimeDeps,
  extraLibraryPath ? "",
}:

let
  libraryPath = lib.concatStringsSep ":" (
    lib.filter (path: path != "") [
      (lib.makeLibraryPath runtimeDeps)
      extraLibraryPath
    ]
  );
in
stdenvNoCC.mkDerivation {
  inherit pname version;

  src = fetchurl {
    inherit (artifact) url hash;
  };

  nativeBuildInputs = [
    autoPatchelfHook
    makeWrapper
  ];
  buildInputs = runtimeDeps;
  autoPatchelfIgnoreMissingDeps = [ "libcuda.so.1" ];

  dontUnpack = true;

  installPhase = ''
    runHook preInstall

    mkdir -p unpacked "$out/bin" "$out/share/doc/${pname}"
    tar -xzf "$src" -C unpacked
    install -m755 "unpacked/bin/${executable}" "$out/bin/${executable}"
    install -m644 unpacked/README.md unpacked/CHANGELOG.md "$out/share/doc/${pname}/"

    wrapProgram "$out/bin/${executable}" \
      --prefix LD_LIBRARY_PATH : "${libraryPath}"

    runHook postInstall
  '';

  meta = {
    description = "Offline speech-to-text with Whisper";
    homepage = "https://github.com/Fractal-Tess/shadoword";
    license = lib.licenses.mit;
    mainProgram = executable;
    platforms = [ "x86_64-linux" ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
}
