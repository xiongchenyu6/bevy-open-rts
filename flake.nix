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

      # NixOS module for easy deployment (serves the prebuilt web bundle).
      flake.nixosModules.default = ./nixos/nginx-module.nix;

      # Overlay so the NixOS module can reference pkgs.bevy-open-rts-web.
      flake.overlays.default = final: prev: {
        bevy-open-rts-web = final.callPackage ./packages/web.nix { };
      };

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
            libxkbcommon
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
          # Prebuilt wasm bundle fetched from GitHub Releases (fast, no Rust
          # toolchain). See packages/web.nix + packages/web-release.json.
          packages.web = pkgs.callPackage ./packages/web.nix { };

          # Source build: compile the game to wasm locally (used by CI to
          # produce the bundle, and for local iteration without a release).
          packages.web-source = pkgs.callPackage ./packages/web-source.nix { };

          packages.default = config.packages.web;

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
