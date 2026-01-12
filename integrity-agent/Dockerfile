# Build stage
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY integrity-common ./integrity-common
COPY integrity-agent ./integrity-agent

# Build the integrity agent
RUN cd integrity-agent && \
    cargo build --release --bin integrity-agent

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc

# Create non-root user (though we'll run privileged for fanotify)
RUN addgroup -g 1000 acropole && \
    adduser -D -u 1000 -G acropole acropole

# Copy the binary from builder
COPY --from=builder /app/target/release/integrity-agent /usr/local/bin/integrity-agent

# Create directory for configuration
RUN mkdir -p /etc/acropole && \
    chown -R acropole:acropole /etc/acropole

# Switch to non-root user
USER acropole

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/integrity-agent"]

# Default command (can be overridden)
CMD ["--help"]
