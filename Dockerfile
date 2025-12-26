# Aegis-Flow Proxy - Production Dockerfile
# Multi-stage build for minimal final image

# Stage 1: Build
FROM rust:nightly-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release --workspace

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 aegis

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/aegis-proxy /app/aegis-proxy

# Set ownership
RUN chown -R aegis:aegis /app

USER aegis

# Expose default port
EXPOSE 8443

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/app/aegis-proxy", "--health"] || exit 1

# Run the proxy
ENTRYPOINT ["/app/aegis-proxy"]
