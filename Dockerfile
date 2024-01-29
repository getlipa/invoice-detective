FROM rust:1.72 as builder

WORKDIR /crates
COPY ./ ./
RUN cargo build --release

FROM rust:1.72
WORKDIR /server
COPY --from=builder /crates/target/release/server .
COPY --from=builder /crates/templates ./templates
COPY --from=builder /crates/graph.db3 .

CMD ["/server/server"]
