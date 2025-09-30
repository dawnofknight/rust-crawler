FROM --platform=$BUILDPLATFORM rust:latest as builder

WORKDIR /usr/src/app
COPY . .

# Build the application with release optimizations
RUN cargo build --release

# Use a debian-based runtime image
FROM --platform=$TARGETPLATFORM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary and migrations from builder
COPY --from=builder /usr/src/app/target/release/rust-postgres-api /app/
COPY --from=builder /usr/src/app/migrations /app/migrations/

# Set the binary as the entrypoint
ENTRYPOINT ["/app/rust-postgres-api"]