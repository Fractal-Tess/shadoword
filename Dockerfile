# syntax=docker/dockerfile:1.7

ARG RUST_IMAGE=rust:1-trixie
ARG RUNTIME_IMAGE=debian:trixie-slim

FROM ${RUST_IMAGE} AS chef

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    clang \
    cmake \
    git \
    glslc \
    glslang-tools \
    libasound2-dev \
    libssl-dev \
    libvulkan-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install --locked cargo-chef

WORKDIR /app

FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY crates/shadoword-core/Cargo.toml crates/shadoword-core/Cargo.toml
COPY crates/shadoword-daemon/Cargo.toml crates/shadoword-daemon/Cargo.toml
COPY crates/shadoword-desktop/Cargo.toml crates/shadoword-desktop/Cargo.toml
COPY crates/shadoword-core/src/lib.rs crates/shadoword-core/src/lib.rs
COPY crates/shadoword-daemon/src/main.rs crates/shadoword-daemon/src/main.rs
COPY crates/shadoword-desktop/src/main.rs crates/shadoword-desktop/src/main.rs

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

ARG CARGO_FEATURES=whisper-vulkan

COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json --package shadoword-daemon --features "${CARGO_FEATURES}"

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p shadoword-daemon --features "${CARGO_FEATURES}"

FROM ${RUNTIME_IMAGE} AS runtime-vulkan

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libasound2 \
    libgcc-s1 \
    libssl3 \
    libstdc++6 \
    libvulkan1 \
    && rm -rf /var/lib/apt/lists/*

ENV XDG_CONFIG_HOME=/config \
    XDG_DATA_HOME=/data \
    RUST_LOG=info

RUN useradd --create-home --home-dir /home/shadoword --shell /usr/sbin/nologin shadoword \
    && mkdir -p /config /data \
    && chown -R shadoword:shadoword /config /data /home/shadoword

COPY --from=builder /app/target/release/shadoword-daemon /usr/local/bin/shadoword-daemon

USER shadoword
WORKDIR /home/shadoword

EXPOSE 47813

ENTRYPOINT ["/usr/local/bin/shadoword-daemon"]
