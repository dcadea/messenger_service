# Messenger Service

This is a Rust-based project that uses various libraries and frameworks to implement a messenger service.

## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing
purposes.

### Prerequisites

- :crab: [Rust](https://www.rust-lang.org/tools/install) installed on your machine.
- :whale: [Docker](https://www.docker.com/get-started) to run dependant services.
- :fuelpump: [Diesel](https://diesel.rs) to run and manage DB migrations.
- :gear: [Make](https://www.gnu.org/software/make/) to run the project with `make` commands.
- :closed_lock_with_key: Have an `Application` configured in the `Authorization Server` of your choice (ex. Auth0, Okta, Google, etc).<br>
Check [configuration](#configuration) for further steps.

### Installing
```bash
# Clone the repository
git clone https://github.com/dcadea/messenger_service.git
cd messenger_service    # Navigate to the project directory
cargo build         # Build the project
```

### Running and testing
```bash
cargo run           # Run the project
cargo watch -x run  # Run the project with hot-reload
                    # (req: `cargo install cargo-watch`)
cargo test          # Run the tests
```
Optionally you can run the project with `cargo run --release` to enable optimizations.<br>
To run the project in **debug mode**, you can use `RUST_LOG=debug cargo run`.<br>
Or you could just use an IDE like RustRover or Zed :rocket:.

### With Docker
```bash
cd messenger_service
docker build -t messenger_service:latest .
docker run -d -p 8000:8000 messenger_service:latest
```

### With Make
```bash
make dev            # Run the service with hot-reload locally
make image          # Build the Docker image
make compose        # Run the service with Docker Compose (dev mode)
make prod           # Run the service with Docker Compose (prod mode)
```

### Configuration
Application will not start without the **required** environment configuration. <br>
**Optional** variables have default values, but it is highly recommended to override these once you have a working setup.
- Required environment variables:
```dotenv
# example with Auth0 as auth server
CLIENT_ID={{your_client_id}}
CLIENT_SECRET={{your_client_secret}}
REDIRECT_URL=http://localhost:8000/callback
ISSUER=https://dcadea.auth0.com/
AUDIENCE=https://messenger.angelwing.io/api/v1
REQUIRED_CLAIMS=iss,sub,aud,exp,permissions
```
- Optional environment variables:
```dotenv
# supported values: local, dev, stg, prod
ENV=local

# required for ENV=prod
SSL_CERT_FILE=/etc/ssl/certs/messenger_service_cert.pem
SSL_KEY_FILE=/etc/ssl/private/messenger_service.pem

# required for ENV=stage and ENV=prod
ALLOW_ORIGIN=https://messenger-app.domain

RUST_LOG=info
SERVICE_NAME=messenger_service

TOKEN_TTL=3600

REDIS_HOST=127.0.0.1
REDIS_PORT=6379

POSTGRES_HOST=127.0.0.1
POSTGRES_PORT=5432
POSTGRES_DB=messenger
POSTGRES_USER={redacted}
POSTGRES_PASSWORD={redacted}

NATS_HOST=127.0.0.1
NATS_PORT=4222
```

### Add new migrations
```bash
# This will generate up.sql and down.sql in ./migrations folder
# which have to be populated with respective sql scripts.
diesel migration generate create_posts

# Once done run the following command to populate schema.rs with actual mapping
diesel migration run
```

## Built With

- [Rust](https://www.rust-lang.org/) - The programming language used
- [Cargo](https://doc.rust-lang.org/cargo/) - The Rust package manager
- [Axum](https://docs.rs/axum/0.7.5/axum/) - Web application framework
- [Tokio](https://tokio.rs/) - Asynchronous runtime
- [htmx](https://htmx.org/) - Hypermedia driven web framework
- [maud](https://maud.lambda.xyz/) - HTML template engine
- [PostgreSQL](https://www.postgresql.org) - Database
- [Redis](https://redis.io/) - In-memory data structure store
- [NATS](https://nats.io) - Pub/Sub messaging system and not only :)

## Authors

- **dcadea** - *Initial work* - [dcadea](https://github.com/dcadea)

## Acknowledgments

- Hat tip to anyone whose code was used
- Inspiration
- etc
