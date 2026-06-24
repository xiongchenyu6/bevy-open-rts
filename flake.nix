# SPDX-FileCopyrightText: 2021 Serokell <https://serokell.io/>
#
# SPDX-License-Identifier: CC0-1.0
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    { nixpkgs, flake-parts, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          lib,
          ...
        }:
        let
          desktopRuntimeLibraries = with pkgs; [
            alsa-lib
            libglvnd
            libxcb
            libxkbcommon
            libx11
            libxcursor
            libxext
            libxi
            libxrandr
            udev
            vulkan-loader
            wayland
          ];

          wasm-bindgen-cli-0_2_125 = pkgs.rustPlatform.buildRustPackage rec {
            pname = "wasm-bindgen-cli";
            version = "0.2.125";

            src = pkgs.fetchCrate {
              inherit pname version;
              hash = "sha256-zRawtjxMOdTMX+mZaiNR3YYfTiZJhf9qj7kXSSeMxrc=";
            };

            cargoHash = "sha256-aZCfgR23Qb0Pn4Mm4ToMtuuRQqSJjXCR9li/VvP5CTM=";
            doCheck = false;
          };
        in
        {
          devShells.default =
            with pkgs;
            mkShell.override { stdenv = pkgs.clangStdenv; } {
              RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
              RUST_BACKTRACE = 1;
              LD_LIBRARY_PATH = lib.makeLibraryPath desktopRuntimeLibraries;

              buildInputs = desktopRuntimeLibraries;
              nativeBuildInputs = [
                pkg-config
                nixfmt
                nixd
                rustc
                cargo
                rust-analyzer
                clippy
                openssl
                rustfmt
                lld
                wasm-bindgen-cli-0_2_125
                binaryen
                python3
              ];
            };
        };
    };
}
