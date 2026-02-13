FROM rust:alpine3.20 AS chef
RUN apk add --no-cache musl-dev gcc openssl-dev openssl-libs-static pkgconfig
# Install nightly toolchain
RUN rustup toolchain install nightly && \
    rustup default nightly
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build app
COPY . .
RUN cargo build --release
RUN strip target/release/temper

# Minimal runtime
FROM alpine:3.20
WORKDIR /app
RUN addgroup -S temper && adduser -S temper -G temper
COPY --from=builder /app/target/release/temper /app/
RUN chown -R temper:temper /app

USER temper
EXPOSE 25565
HEALTHCHECK --interval=30s --timeout=3s --start-period=40s \
  CMD nc -z localhost 25565 || exit 1
CMD ["./temper"]
