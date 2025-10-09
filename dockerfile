# Chef planner stage
FROM lukemathwalker/cargo-chef:latest-rust-1.90.0 AS chef
WORKDIR /app

# Install OpenSSL development package
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Create recipe.json
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:trixie-slim

# Install CA certificates and OpenSSL
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/aws-vpc-prefix-list-monitor /usr/local/bin/monitor

# Create non-root user
RUN useradd -m -u 1000 monitor && \
    chown monitor:monitor /usr/local/bin/monitor

USER monitor

# Set environment variables with defaults
ENV AWS_REGION=us-east-1 \
    CHECK_INTERVAL=300 \
    CIDR_SUFFIX=32 \
    IP_SERVICE_URL=https://api.ipify.org \
    RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/monitor"]