# Build stage
FROM rust:1.83-slim as builder

# Install required dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY config ./config

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/manti /app/manti

# Copy migrations
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/config /app/config

# Create non-root user
RUN useradd -m -u 1001 manti && chown -R manti:manti /app
USER manti

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV MANTI_SERVER_HOST=0.0.0.0
ENV MANTI_SERVER_PORT=8080

# Run the application
CMD ["./manti"]
