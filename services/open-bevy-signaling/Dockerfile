FROM rust:1.95-bookworm AS builder

WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY crates/open-bevy-net ./crates/open-bevy-net
COPY crates/open-bevy-protocol ./crates/open-bevy-protocol
COPY services/open-bevy-signaling ./services/open-bevy-signaling
RUN cargo build --locked --release -p open-bevy-signaling

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --no-create-home open-bevy
COPY --from=builder /source/target/release/open-bevy-signaling /usr/local/bin/open-bevy-signaling

USER open-bevy
EXPOSE 3536
HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl --fail --silent http://127.0.0.1:3536/healthz >/dev/null || exit 1
ENTRYPOINT ["/usr/local/bin/open-bevy-signaling"]
