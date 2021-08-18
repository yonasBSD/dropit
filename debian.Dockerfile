FROM rust:1.54-slim AS builder

# Required by sqlx even if we don't use any SSL connection.
RUN apt-get update && apt-get install -y openssl libssl-dev pkg-config

WORKDIR /app
COPY . .
RUN cargo build --release

#------------

FROM debian:buster-slim

COPY --from=builder /app/target/release/dropit /dropit

ENTRYPOINT ["/dropit"]