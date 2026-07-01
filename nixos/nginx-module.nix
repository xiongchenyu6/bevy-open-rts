# NixOS module for deploying Bevy Open RTS with Nginx.
#
# Usage:
#   {
#     imports = [ inputs.bevy-open-rts.nixosModules.default ];
#     services.bevy-open-rts = {
#       enable = true;
#       hostname = "rts.example.com";
#       enableACME = true;            # HTTPS via ACME/Let's Encrypt
#       extraConfig = ''             # extra server-block directives
#         send_timeout 300s;
#       '';
#     };
#   }
#
# Serves the prebuilt WebGPU/wasm bundle (pkgs.bevy-open-rts-web) with the right
# wasm MIME type, immutable caching for the hashed pkg files, and the raw-gzip
# handling the loader expects.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.bevy-open-rts;
  package = cfg.package;

  # nginx's `add_header` REPLACES all parent-level headers in a location that
  # declares its own `add_header` — so every location below that sets a header
  # must re-state these, or they silently vanish there. (gixy, which srvos runs
  # at build time, also fails the build on this redefinition.)
  securityHeaders = ''
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header Cross-Origin-Embedder-Policy "require-corp" always;
    add_header Cross-Origin-Opener-Policy "same-origin" always;
  '';
in
{
  options.services.bevy-open-rts = {
    enable = lib.mkEnableOption "Bevy Open RTS web game";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.bevy-open-rts-web or (pkgs.callPackage ../packages/web.nix { });
      defaultText = lib.literalExpression "pkgs.bevy-open-rts-web";
      description = "The bevy-open-rts-web package to serve.";
    };

    hostname = lib.mkOption {
      type = lib.types.str;
      default = "localhost";
      description = "Hostname for the Nginx virtual host.";
    };

    listenPort = lib.mkOption {
      type = lib.types.port;
      default = 80;
      description = "Port for the Nginx virtual host to listen on.";
    };

    enableACME = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable HTTPS via ACME (Let's Encrypt). Requires a valid domain.";
    };

    forceSSL = lib.mkOption {
      type = lib.types.bool;
      default = cfg.enableACME;
      description = "Force HTTPS redirect. Automatically enabled when enableACME is true.";
    };

    extraConfig = lib.mkOption {
      type = lib.types.lines;
      default = "";
      description = "Extra Nginx server block directives.";
    };
  };

  config = lib.mkIf cfg.enable {
    services.nginx.enable = lib.mkDefault true;

    # Correct MIME type for .wasm files.
    services.nginx.appendHttpConfig = ''
      types {
        application/wasm wasm;
      }
    '';

    services.nginx.virtualHosts."${cfg.hostname}" = {
      root = "${package}/share/bevy-open-rts";

      # `addr` is required by newer nixpkgs (the listen submodule has no default
      # for it), so set it explicitly on every entry.
      listen = lib.mkMerge [
        [
          {
            addr = "0.0.0.0";
            port = cfg.listenPort;
            ssl = false;
          }
          {
            addr = "[::]";
            port = cfg.listenPort;
            ssl = false;
          }
        ]
        (lib.mkIf cfg.enableACME [
          {
            addr = "0.0.0.0";
            port = 443;
            ssl = true;
          }
          {
            addr = "[::]";
            port = 443;
            ssl = true;
          }
        ])
      ];

      extraConfig = ''
        # --- Compression (dynamic, for the html/css/json; wasm/js ship precompressed) ---
        gzip on;
        gzip_types
          application/javascript
          application/json
          application/wasm
          text/css
          text/html
          text/plain
          text/xml;
        gzip_vary on;

        # --- Security headers ---
        # Applied at server level for locations that add no headers of their own;
        # locations below that DO add headers must repeat these (add_header is
        # replace-all, not additive).
        ${securityHeaders}

        # Hashed wasm-bindgen output under /pkg — cache immutably.
        location ~* \.(wasm|js)$ {
          ${securityHeaders}
          add_header Cache-Control "public, max-age=31536000, immutable";
          add_header Access-Control-Allow-Origin "*";
        }

        # The loader fetches bevy_open_rts_bg.wasm.gz and inflates it in JS via
        # DecompressionStream, so this MUST be served raw — no Content-Encoding,
        # or the browser would decode it first and the manual gunzip would then
        # fail on already-plain bytes. Disable re-compression of the .gz too.
        location ~* \.wasm\.gz$ {
          ${securityHeaders}
          add_header Cache-Control "public, max-age=31536000, immutable";
          add_header Access-Control-Allow-Origin "*";
          gzip off;
        }

        # Game assets (models, textures, audio, fonts) — cache a day.
        location /assets/ {
          ${securityHeaders}
          add_header Cache-Control "public, max-age=86400";
        }

        # index.html should NOT be cached — it carries the cache-busting buildId.
        location = /index.html {
          ${securityHeaders}
          add_header Cache-Control "no-cache, must-revalidate";
        }

        # Fallback for SPA-style routing.
        location / {
          try_files $uri $uri/ /index.html;
        }

        ${cfg.extraConfig}
      '';

      enableACME = cfg.enableACME;
      forceSSL = cfg.forceSSL;
    };
  };
}
