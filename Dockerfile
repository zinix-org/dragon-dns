FROM rust:latest AS builder
WORKDIR /app

# dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
RUN rm -rf src

COPY . .
RUN cargo build --release

FROM debian:trixie-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/dragon-dns /ddns
ENTRYPOINT ["/ddns"]
