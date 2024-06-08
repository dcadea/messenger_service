# Messenger API

This is a Rust-based project that uses various libraries and frameworks to implement a messenger API.

## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing
purposes.

### Prerequisites

- :crab: [Rust](https://www.rust-lang.org/tools/install) installed on your machine.
- :whale: [Docker](https://www.docker.com/get-started) to run dependant services.
- :closed_lock_with_key: Have a `Application` configured in the `Authorization Server` of your choice (ex. Auth0, Okta, Google, etc).<br>
Check [configuration](#configuration) for further steps.

### Installing
```bash
# Clone the repository
git clone https://github.com/dcadea/messenger_api.git
cd messenger_api    # Navigate to the project directory
cargo build         # Build the project
```

### With Docker
```bash
cd messenger_api
docker build -t messenger_api:latest .
docker run -d -p 8000:8000 messenger_api:latest
```

### Running and testing
```bash
cargo run   # Run the project
cargo test  # Run the tests
```
Optionally you can run the project with `cargo run --release` to enable optimizations.<br>
To run the project in **debug mode**, you can use `RUST_LOG=debug cargo run`.<br>
Or you could just use an IDE like RustRover or Zed :rocket:.

### Configuration
Application will not start without the **required** environment configuration. <br>
**Optional** variables have default values, but it is highly recommended to override these once you have a working setup.
- Required environment variables:
```dotenv
# example with Auth0 as auth server
ISSUER=https://dcadea.auth0.com/
AUDIENCE=https://messenger.angelwing.io/api/v1
REQUIRED_CLAIMS=iss,sub,aud,exp,permissions
```
- Optional environment variables:
```dotenv
REDIS_HOST=127.0.0.1
REDIS_PORT=6379

MONGO_USERNAME=root
MONGO_PASSWORD=example
MONGO_HOST=127.0.0.1
MONGO_PORT=27017
MONGO_DB=messenger

AMQP_URI=amqp://127.0.0.1:5672/%2f
```

## Built With

- [Rust](https://www.rust-lang.org/) - The programming language used
- [Cargo](https://doc.rust-lang.org/cargo/) - The Rust package manager
- [Axum](https://docs.rs/axum/0.7.5/axum/) - Web application framework
- [Tokio](https://tokio.rs/) - Asynchronous runtime
- [MongoDB](https://www.mongodb.com/) - Database
- [Redis](https://redis.io/) - In-memory data structure store
- [Lapin](https://docs.rs/lapin/2.3.4/lapin/) - AMQP client library

## Authors

- **dcadea** - *Initial work* - [dcadea](https://github.com/dcadea)

## Acknowledgments

- Hat tip to anyone whose code was used
- Inspiration
- etc