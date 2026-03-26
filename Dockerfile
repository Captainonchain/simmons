# Build stage
FROM rust:latest AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/simmons /app/simmons

# Copy config directory
COPY config ./config

# Create data directory with empty files
RUN mkdir -p data && \
    echo '[]' > data/trades.json && \
    echo '{}' > data/decision.json && \
    echo '{}' > data/dual_brain_context.json

# Expose port
EXPOSE 3456

# Run
CMD ["./simmons", "dual", "--dashboard"]
