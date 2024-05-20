FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates sqlite3
RUN update-ca-certificates

COPY graph.db3 .
COPY templates/ templates/
COPY target/release/server .

CMD ["./server"]
