# syntax=docker/dockerfile:1.7-labs

FROM rust:1.91 AS chef
RUN apt-get update \
    && apt-get install -y --no-install-recommends clang mold pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN --mount=type=cache,id=mlm-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=mlm-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    cargo install cargo-chef --locked \
    && cargo install dioxus-cli --version 0.7.3 --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY server/Cargo.toml server/Cargo.toml
COPY server/build.rs server/build.rs
COPY mlm_web_api/Cargo.toml mlm_web_api/Cargo.toml
COPY mlm_db/Cargo.toml mlm_db/Cargo.toml
COPY mlm_parse/Cargo.toml mlm_parse/Cargo.toml
COPY mlm_mam/Cargo.toml mlm_mam/Cargo.toml
COPY mlm_meta/Cargo.toml mlm_meta/Cargo.toml
COPY mlm_core/Cargo.toml mlm_core/Cargo.toml
COPY mlm_web_askama/Cargo.toml mlm_web_askama/Cargo.toml
COPY mlm_web_dioxus/Cargo.toml mlm_web_dioxus/Cargo.toml
RUN mkdir -p \
        server/src/bin \
        mlm_web_api/src \
        mlm_db/src \
        mlm_parse/src \
        mlm_mam/src \
        mlm_meta/src \
        mlm_core/src \
        mlm_web_askama/src \
        mlm_web_dioxus/src \
    && touch \
        server/src/lib.rs \
        server/src/main.rs \
        server/src/bin/create_test_db.rs \
        server/src/bin/libation_unmapped_categories.rs \
        server/src/bin/mock_server.rs \
        mlm_web_api/src/lib.rs \
        mlm_db/src/lib.rs \
        mlm_parse/src/lib.rs \
        mlm_mam/src/lib.rs \
        mlm_meta/src/lib.rs \
        mlm_core/src/lib.rs \
        mlm_web_askama/src/lib.rs \
        mlm_web_dioxus/src/lib.rs \
        mlm_web_dioxus/src/main.rs \
    && cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,id=mlm-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=mlm-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=mlm-target,target=/app/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY server server
COPY mlm_web_api mlm_web_api
COPY mlm_db mlm_db
COPY mlm_parse mlm_parse
COPY mlm_mam mlm_mam
COPY mlm_meta mlm_meta
COPY mlm_core mlm_core
COPY mlm_web_askama mlm_web_askama
COPY mlm_web_dioxus mlm_web_dioxus
RUN --mount=type=cache,id=mlm-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=mlm-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=mlm-target,target=/app/target,sharing=locked \
    cargo build --release --bin mlm && \
    cp /app/target/release/mlm /app/mlm
RUN --mount=type=cache,id=mlm-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=mlm-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=mlm-target,target=/app/target,sharing=locked \
    cd /app/mlm_web_dioxus && /usr/local/cargo/bin/dx build --release --fullstack --skip-assets && \
    mkdir -p /app/dx_output && cp -r /app/target/dx/mlm_web_dioxus /app/dx_output/

FROM debian:trixie-slim AS app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 mlm \
    && useradd --uid 1000 --gid 1000 --shell /usr/sbin/nologin mlm \
    && mkdir -p /data /config /dioxus-public \
    && chown -R mlm:mlm /data /config /dioxus-public
COPY --chown=mlm:mlm server/assets /server/assets
COPY --chown=mlm:mlm entrypoint.sh /entrypoint.sh
COPY --from=builder /app/mlm /mlm
COPY --from=builder /app/dx_output/mlm_web_dioxus/release/web/public /dioxus-public
ENV MLM_LOG_DIR=""
ENV MLM_CONFIG_FILE="/config/config.toml"
ENV MLM_DB_FILE="/data/data.db"
ENV DIOXUS_PUBLIC_PATH="/dioxus-public"
USER mlm
ENTRYPOINT ["/entrypoint.sh"]
CMD ["/mlm"]
