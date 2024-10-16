dev:
	cargo watch -x "run --bin messenger_service" -w src

build:
	cargo build --verbose
