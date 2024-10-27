all: image compose

dev:
	cargo watch -x "run --bin messenger_service" -w src

image:
	docker build -t messenger_service:latest .

compose:
	docker-compose -f docker-compose.dev.yaml up
