FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS test_runner
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --tests --recipe-path recipe.json
# Build and run tests
COPY . .
RUN cargo test --release 

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --bin receipt-rs --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin receipt-rs

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt install -y openssl
WORKDIR /app
COPY --from=builder /app/target/release/receipt-rs /usr/local/bin
ENTRYPOINT ["/usr/local/bin/receipt-rs"]
