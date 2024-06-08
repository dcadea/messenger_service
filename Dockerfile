FROM rust:1.78 as builder
LABEL authors="dcadea"

WORKDIR /usr/src/messenger_api
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
LABEL authors="dcadea"
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/messenger_api/target/release/messenger_api /usr/local/bin/messenger_api

EXPOSE 8000

CMD ["messenger_api"]
