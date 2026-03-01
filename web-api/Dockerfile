# Multi-stage build for Cilium Vision API
# Stage 1: Build (use bookworm to match runtime glibc)
FROM rust:bookworm AS builder

WORKDIR /build

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and lock file for reproducible builds
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build for release
RUN cargo build --release

# Stage 2: Runtime (same Debian release as builder)
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/cilium-vision-api ./

# Change ownership
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 9191

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT:-9191}/health || exit 1

# Run
CMD ["./cilium-vision-api"]
