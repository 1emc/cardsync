FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo install cargo-audit --locked
RUN cargo audit
RUN cargo build --release -p api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/galcard-api
COPY migrations ./migrations
EXPOSE 3000
CMD ["galcard-api"]
