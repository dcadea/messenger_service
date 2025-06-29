all: image compose

image:
	docker build -t messenger_service:latest .

dev:
	cargo watch -x "run --bin messenger_service" -w src

prod: image
	docker-compose -f docker-compose.yaml up -d

stg: image
	docker-compose -f docker-compose.stg.yaml up -d

compose:
	docker-compose -f docker-compose.dev.yaml up -d

migrate:
	diesel migration run

clippy:
	cargo fmt && cargo clippy -- \
	-W clippy::pedantic \
	-W clippy::nursery \
	-W clippy::unwrap_used

cov:
	cargo llvm-cov --open
