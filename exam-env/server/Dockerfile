# Build context is exam-env/ (the workspace), not this directory.
#   docker build -f server/Dockerfile -t exam-server .

FROM rust:1-bookworm AS build

# One rustc per CPU will exhaust an 8GB Docker VM while linking.
ENV CARGO_BUILD_JOBS=2

RUN rustup target add wasm32-unknown-unknown
RUN cargo install cargo-leptos --version 0.3.11 --locked

WORKDIR /src
COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY server ./server
COPY seb-agent ./seb-agent
COPY sim-agent ./sim-agent

# .cargo/config.toml enables erase_components. Without it the Leptos
# release build can exhaust memory while linking.
WORKDIR /src/server
RUN cargo leptos build --release

RUN set -eu; \
    site=""; \
    for d in /src/server/target/site /src/target/site; do \
      if [ -d "$d/pkg" ] && ls "$d/pkg"/*.wasm >/dev/null 2>&1; then site="$d"; break; fi; \
    done; \
    if [ -z "$site" ]; then echo "missing leptos site assets"; exit 1; fi; \
    mkdir -p /out/site; \
    cp -a "$site"/. /out/site/; \
    if [ -x /src/target/release/server ]; then cp /src/target/release/server /out/server; \
    elif [ -x /src/server/target/release/server ]; then cp /src/server/target/release/server /out/server; \
    else echo "missing server binary"; exit 1; fi

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 -s /usr/sbin/nologin server \
    && mkdir -p /data /app/site \
    && chown server:server /data

COPY --from=build /out/server /usr/local/bin/server
COPY --from=build /out/site /app/site
COPY server/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod 755 /usr/local/bin/docker-entrypoint.sh

WORKDIR /app
ENV LEPTOS_OUTPUT_NAME=console \
    LEPTOS_SITE_ROOT=/app/site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:8000 \
    LEPTOS_ENV=prod \
    EXAM_DATA_DIR=/data
EXPOSE 8000
ENTRYPOINT ["docker-entrypoint.sh"]
