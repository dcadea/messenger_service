FROM rust:1.83 AS builder
LABEL authors="dcadea"

WORKDIR /usr/src/messenger_service
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
LABEL authors="dcadea"
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/messenger_service/target/release/messenger_service /usr/local/bin/messenger_service

EXPOSE 8000

CMD ["messenger_service"]
