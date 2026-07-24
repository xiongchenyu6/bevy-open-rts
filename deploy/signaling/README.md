# Signaling Deployment

1. Point `SIGNALING_HOST` and `TURN_PUBLIC_HOST` DNS records at the host.
2. Copy `.env.example` to `.env` and replace every example value. Generate the
   shared secret with `openssl rand -hex 32`.
3. Open TCP 80/443/3478, UDP 3478, and UDP 49160-49200 in the host firewall and
   cloud security group.
4. Start the stack from this directory:

   ```sh
   docker compose up -d --build
   docker compose ps
   curl "https://${SIGNALING_HOST}/healthz"
   ```

5. Confirm `/v1/config` returns both STUN and time-limited TURN entries.

The Caddy access log redacts request URIs so host/join tickets are not written to
disk. Keep that behavior when replacing Caddy with another edge proxy.
