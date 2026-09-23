# Multi-stage build for Paqtra API
# Stage 1: Build (use bookworm to match runtime glibc)
FROM docker.io/library/rust:bookworm AS builder

WORKDIR /build

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and lock file for dependency caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy real source and build
COPY src/ src/
RUN cargo build --release

# Stage 2: Runtime (same Debian release as builder)
FROM docker.io/library/debian:bookworm-slim

# Runtime deps + kubectl (API shells out to kubectl using in-cluster SA)
ARG TARGETARCH=amd64
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    wget \
    curl \
    && KVER=$(curl -fsSL https://dl.k8s.io/release/stable.txt) \
    && curl -fsSL -o /usr/local/bin/kubectl \
      "https://dl.k8s.io/release/${KVER}/bin/linux/${TARGETARCH}/kubectl" \
    && chmod +x /usr/local/bin/kubectl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/paqtra-api ./

# Change ownership; /data holds the SQLite state when PAQTRA_DATA_DIR=/data
RUN mkdir -p /data && chown -R appuser:appuser /app /data

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 9191

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:9191/health || exit 1

# Run
CMD ["./paqtra-api"]
