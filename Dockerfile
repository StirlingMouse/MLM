# syntax=docker/dockerfile:1.3-labs

# The above line is so we can use can use heredocs in Dockerfiles. No more && and \!
# https://www.docker.com/blog/introduction-to-heredocs-in-dockerfiles/

FROM rust:1.91 AS build

RUN cargo new --lib app/mlm_db
RUN cargo new --lib app/mlm_parse
RUN cargo new --bin app/server

# Capture dependencies
COPY Cargo.toml Cargo.lock /app/
COPY mlm_db/Cargo.toml /app/mlm_db/
COPY mlm_parse/Cargo.toml /app/mlm_parse/
COPY server/Cargo.toml /app/server/

# This step compiles only our dependencies and saves them in a layer. This is the most impactful time savings
# Note the use of --mount=type=cache. On subsequent runs, we'll have the crates already downloaded
WORKDIR /app
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

# Copy our sources
COPY ./mlm_db /app/mlm_db
COPY ./mlm_parse /app/mlm_parse
COPY ./server /app/server

# A bit of magic here!
# * We're mounting that cache again to use during the build, otherwise it's not present and we'll have to download those again - bad!
# * EOF syntax is neat but not without its drawbacks. We need to `set -e`, otherwise a failing command is going to continue on
# * Rust here is a bit fiddly, so we'll touch the files (even though we copied over them) to force a new build
RUN --mount=type=cache,target=/usr/local/cargo/registry <<EOF
  set -e
  # update timestamps to force a new build
  touch /app/mlm_db/src/lib.rs
  touch /app/mlm_parse/src/lib.rs
  touch /app/server/src/main.rs
  cargo build --release
EOF

CMD ["/app/target/release/mlm"]

# Again, our final image is the same - a slim base and just our app
FROM debian:trixie-slim AS app
RUN apt update && apt install -y ca-certificates && apt clean
COPY ./server/assets /server/assets
COPY --from=build /app/target/release/mlm /mlm
ENV MLM_LOG_DIR=""
ENV MLM_CONFIG_FILE="/config/config.toml"
ENV MLM_DB_FILE="/data/data.db"
CMD ["/mlm"]
