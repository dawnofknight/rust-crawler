FROM --platform=$BUILDPLATFORM rust:latest as builder

WORKDIR /usr/src/app
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config cmake make gcc g++ \
    libssl-dev zlib1g-dev libsasl2-dev && \
    rm -rf /var/lib/apt/lists/*
COPY . .

# Build the application with release optimizations
RUN cargo build --release

# Use a debian-based runtime image
FROM --platform=$TARGETPLATFORM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libsasl2-2 zlib1g libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary and migrations from builder
COPY --from=builder /usr/src/app/target/release/rust-postgres-api /app/
COPY --from=builder /usr/src/app/migrations /app/migrations/
COPY --from=builder /usr/src/app/.env.example /app/.env

# Expose the API port
EXPOSE 8080

# Set the binary as the entrypoint
ENTRYPOINT ["/app/rust-postgres-api"]