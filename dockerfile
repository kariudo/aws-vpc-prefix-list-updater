# Build stage
FROM rust:1.83-slim as builder

WORKDIR /app

# Install OpenSSL development package
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src

# Build the actual binary
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

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