FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libasound2 \
    libgcc-s1 \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

COPY docker/rootfs/ /

ENV XDG_CONFIG_HOME=/config \
    XDG_DATA_HOME=/data \
    SHADOWORD_LISTEN_ADDR=0.0.0.0:47813 \
    RUST_LOG=info

RUN useradd --create-home --home-dir /home/shadoword --shell /usr/sbin/nologin shadoword \
    && mkdir -p /config /data \
    && chown -R shadoword:shadoword /config /data /home/shadoword

USER shadoword
WORKDIR /home/shadoword

EXPOSE 47813

ENTRYPOINT ["/usr/local/bin/shadoword-daemon"]
