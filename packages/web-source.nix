# Build the Bevy Open RTS WebGPU/wasm bundle from source.
#
# Mirrors scripts/build_web.sh: cargo build (wasm32, webgpu) → wasm-bindgen →
# wasm-opt (STABLE post-MVP features only) → gzip → stamp cache-busting build id
# → copy assets. Produces $out/share/bevy-open-rts/{index.html,styles.css,pkg/,assets/}.
#
# The deploy path uses the prebuilt `web` package (packages/web.nix) instead;
# this is here for local `nix build .#web-source` and to document the build.
{
  lib,
  rustPlatform,
  fetchCrate,
  buildWasmBindgenCli,
  pkg-config,
  binaryen,
  gzip,
  lld,
  python3,
  openssl,
  # Bevy native deps aren't linked into the wasm, but cargo/build scripts probe
  # for them while resolving the graph.
  vulkan-loader,
  alsa-lib,
  udev,
  wayland,
  libxkbcommon,
}:
let
  # wasm-bindgen CLI MUST match the wasm-bindgen crate version in Cargo.lock
  # (0.2.126), or the JS glue and wasm disagree and boot fails with
  # "WebAssembly.Table.grow(): failed to grow table by 4".
  wasm-bindgen-cli = buildWasmBindgenCli rec {
    src = fetchCrate {
      pname = "wasm-bindgen-cli";
      version = "0.2.126";
      hash = "sha256-H6Is3fiZVxZCfOMWK5dWMSrtn50VGv0sfdnsT+cTtyk=";
    };
    cargoDeps = rustPlatform.fetchCargoVendor {
      inherit src;
      inherit (src) pname version;
      hash = "sha256-VucqkXbCi4qtQzY/HrXiDnbSURsagPsdNVMn1Tw3UiY=";
    };
  };
in
rustPlatform.buildRustPackage {
  pname = "bevy-open-rts-web";
  version = "0.1.0";
  src = ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = { };
  };

  CARGO_BUILD_TARGET = "wasm32-unknown-unknown";

  nativeBuildInputs = [
    pkg-config
    wasm-bindgen-cli
    binaryen
    gzip
    python3
    lld # wasm32-unknown-unknown linker
  ];

  buildInputs = [
    openssl
    vulkan-loader
    alsa-lib
    udev
    wayland
    libxkbcommon
  ];

  doCheck = false;
  dontCargoBuild = true;
  dontCargoInstall = true;

  buildPhase = ''
    runHook preBuild

    echo ">>> cargo build (release, wasm32-unknown-unknown, webgpu)"
    cargo build --release --target wasm32-unknown-unknown --features webgpu --locked

    echo ">>> wasm-bindgen -> web/pkg"
    rm -rf web/pkg web/assets
    mkdir -p web/pkg
    wasm-bindgen \
      --target web \
      --out-dir web/pkg \
      --out-name bevy_open_rts \
      target/wasm32-unknown-unknown/release/bevy-open-rts.wasm

    echo ">>> wasm-opt -Oz (stable post-MVP features only; NOT --all-features)"
    BG="web/pkg/bevy_open_rts_bg.wasm"
    if [ -f "$BG" ]; then
      wasm-opt -Oz \
        --enable-reference-types \
        --enable-bulk-memory \
        --enable-nontrapping-float-to-int \
        --enable-sign-ext \
        --enable-mutable-globals \
        --enable-multivalue \
        -o "$BG.opt" "$BG" && mv "$BG.opt" "$BG" \
        || echo ">>> wasm-opt failed; shipping un-optimized wasm"
    fi

    echo ">>> gzip wasm (loader streams the .gz with a real progress bar)"
    gzip -9 -kf "$BG"

    echo ">>> stamp cache-busting build id"
    python3 scripts/stamp_web_build_id.py \
      web/index.html \
      web/pkg/bevy_open_rts.js \
      web/pkg/bevy_open_rts_bg.wasm

    echo ">>> copying assets"
    cp -R assets web/assets

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/share/bevy-open-rts
    cp -r web/* $out/share/bevy-open-rts/
    runHook postInstall
  '';

  meta = with lib; {
    description = "Bevy Open RTS — WebGPU/wasm web build (from source)";
    homepage = "https://github.com/xiongchenyu6/bevy-open-rts";
    license = licenses.mit;
    platforms = platforms.linux ++ platforms.darwin;
    maintainers = [ ];
  };
}
