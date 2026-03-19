# Build stage
FROM rust:1.94-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY integrity-common ./integrity-common
COPY metadata-service ./metadata-service

# Create stub directories for other workspace members so Cargo resolves the workspace
RUN mkdir -p baseline-collector/src && echo "fn main() {}" > baseline-collector/src/main.rs
COPY baseline-collector/Cargo.toml ./baseline-collector/Cargo.toml
RUN mkdir -p integrity-agent/src && echo "fn main() {}" > integrity-agent/src/main.rs
COPY integrity-agent/Cargo.toml ./integrity-agent/Cargo.toml

# Build the metadata service
RUN cargo build --release --bin metadata-service

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc

# Create non-root user
RUN addgroup -g 1000 acropole && \
    adduser -D -u 1000 -G acropole acropole

# Copy the binary from builder
COPY --from=builder /app/target/release/metadata-service /usr/local/bin/metadata-service

# Create directory for database
RUN mkdir -p /var/lib/acropole/metadata-db && \
    chown -R acropole:acropole /var/lib/acropole

# Expose port
EXPOSE 8080

# Switch to non-root user
USER acropole

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/metadata-service"]

# Default command
CMD ["--host", "0.0.0.0", "--port", "8080", "--db-path", "/var/lib/acropole/metadata-db"]
