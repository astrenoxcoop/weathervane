# syntax=docker/dockerfile:1.4
FROM rust:1-bookworm AS build

RUN cargo install sccache --version ^0.8
ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache

RUN USER=root cargo new --bin weathervane
RUN mkdir -p /app/
WORKDIR /app/

ARG GIT_HASH
ENV GIT_HASH=$GIT_HASH

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=templates,target=templates \
    --mount=type=bind,source=static,target=static \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=build.rs,target=build.rs \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --locked --release --bin weathervane --target-dir . --no-default-features -F embed
EOF

FROM debian:bookworm-slim

RUN set -x \
    && apt-get update \
    && apt-get install ca-certificates -y

ENV RUST_LOG=info
ENV RUST_BACKTRACE=full

COPY --from=build /app/release/weathervane /var/lib/weathervane/

WORKDIR /var/lib/weathervane

ENTRYPOINT ["sh", "-c", "/var/lib/weathervane/weathervane"]
