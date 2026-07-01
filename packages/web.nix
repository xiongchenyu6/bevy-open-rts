# Prebuilt Bevy Open RTS WebGPU/wasm bundle.
#
# Instead of compiling the game from source on every deploy (slow — minutes of
# Rust plus wasm-opt, plus a ~130 MB asset copy), this fetches the static web
# bundle that CI publishes to GitHub Releases (.github/workflows/release-wasm.yml).
# Every deploy target just downloads the tarball, so `nixos-rebuild` is fast and
# needs no Rust toolchain.
#
# The release coordinates live in ./web-release.json, which CI rewrites whenever
# a new `v*` tag is built. To rebuild from source instead, use the `web-source`
# package (packages/web-source.nix).
#
#   nix build .#web           # download the pinned prebuilt bundle
#   nix build .#web-source    # compile from source
{
  lib,
  stdenvNoCC,
  fetchurl,
  unzip,
}:
let
  release = lib.importJSON ./web-release.json;
  isZip = lib.hasSuffix ".zip" release.url;
in
stdenvNoCC.mkDerivation {
  pname = "bevy-open-rts-web";
  version = lib.removePrefix "v" release.tag;

  src = fetchurl {
    url = release.url;
    sha256 = release.sha256;
  };

  nativeBuildInputs = lib.optionals isZip [ unzip ];

  # CI release artifacts put the bundle files (index.html, styles.css, pkg/…,
  # assets/…) at the archive root with no wrapping directory. The current rolling
  # web-latest release is a zip; release-wasm.yml publishes a tar.gz.
  unpackPhase = ''
    runHook preUnpack
    mkdir -p bundle
    ${if isZip then "unzip -q \"$src\" -d bundle" else "tar -xzf \"$src\" -C bundle"}
    runHook postUnpack
  '';
  sourceRoot = "bundle";

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/share/bevy-open-rts
    cp -r ./* $out/share/bevy-open-rts/
    runHook postInstall
  '';

  meta = with lib; {
    description = "Bevy Open RTS — prebuilt WebGPU/wasm web bundle";
    homepage = "https://github.com/xiongchenyu6/bevy-open-rts";
    license = licenses.mit;
    platforms = platforms.all;
    maintainers = [ ];
  };
}
