# build
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# run — tiny
FROM debian:bookworm-slim
RUN useradd -r -s /usr/sbin/nologin server
COPY --from=build /app/target/release/server /usr/local/bin/server
USER server
EXPOSE 8000
CMD ["server"]
