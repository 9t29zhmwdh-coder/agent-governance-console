# Multi-stage build: compile agc-api in a full Rust image, ship only the
# resulting binary in a minimal Debian runtime. Keeps the final image
# small and avoids shipping the Rust toolchain/build cache to production.

FROM rust:1.90-bookworm AS build
WORKDIR /build
COPY . .
RUN cargo build --release -p agc-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --shell /usr/sbin/nologin agc

COPY --from=build /build/target/release/agc-api /usr/local/bin/agc-api

USER agc
WORKDIR /home/agc
# The default bind address (127.0.0.1) is only reachable from inside
# this container's own network namespace -- neither Docker's port
# mapping nor a Kubernetes Service/probe can reach it. 0.0.0.0 is
# required for a containerized deployment to actually work.
ENV AGC_BIND=0.0.0.0:8080
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/agc-api"]
