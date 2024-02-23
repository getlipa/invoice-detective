FROM rust:1.74 as builder
WORKDIR /crates
COPY ./ ./
RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=/crates/target \
    cargo install --path server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y sqlite3 && rm -rf /var/lib/apt/lists/*
WORKDIR /server
COPY --from=builder /usr/local/cargo/bin/server .
COPY --from=builder /crates/templates ./templates
COPY --from=builder /crates/graph.db3 .
CMD ["/server/server"]
