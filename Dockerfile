# Build stage
FROM rust:1.83-slim-bookworm as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo 'fn main() {}' > src/main.rs

# Build dependencies only (native build includes server code)
RUN cargo build --release

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY assets ./assets

# Touch main.rs to ensure it rebuilds
RUN touch src/main.rs

# Build the application (server code is included for native targets)
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary and assets
COPY --from=builder /app/target/release/voip-crm /app/voip-crm
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/assets /app/assets

# Set environment
ENV PORT=3000
ENV RUST_LOG=info

# Expose port
EXPOSE 3000

# Run the server
CMD ["/app/voip-crm", "--server"]
