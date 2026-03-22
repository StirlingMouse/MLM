# syntax=docker/dockerfile:1.7-labs

FROM rust:1.91 AS chef
RUN apt update && apt install -y clang mold pkg-config && apt clean
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install dioxus-cli --version 0.7.3 --locked
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cd /app/mlm_web_dioxus && dx build --release --fullstack --skip-assets

FROM debian:trixie-slim AS app
RUN apt update && apt install -y ca-certificates && apt clean \
    && groupadd --gid 1000 mlm \
    && useradd --uid 1000 --gid 1000 --shell /usr/sbin/nologin mlm \
    && mkdir -p /data /config /dioxus-public \
    && chown -R mlm:mlm /data /config /dioxus-public
COPY --chown=mlm:mlm ./server/assets /server/assets
COPY --chown=mlm:mlm entrypoint.sh /entrypoint.sh
COPY --from=builder /app/target/release/mlm /mlm
COPY --from=builder /app/target/dx/mlm_web_dioxus/release/web/public /dioxus-public
ENV MLM_LOG_DIR=""
ENV MLM_CONFIG_FILE="/config/config.toml"
ENV MLM_DB_FILE="/data/data.db"
ENV DIOXUS_PUBLIC_PATH="/dioxus-public"
USER mlm
ENTRYPOINT ["/entrypoint.sh"]
CMD ["/mlm"]
