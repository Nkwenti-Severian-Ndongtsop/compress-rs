# Build stage
FROM rust:1.77-slim-bullseye AS builder

WORKDIR /usr/src/app

# Copy only the necessary files for building
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build with optimizations and security flags
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-Wl,-z,now,-z,relro,-z,noexecstack" cargo build --release

# Final stage
FROM scratch

# Copy the binary and certificates from the builder stage
COPY --from=builder /usr/src/app/target/release/rszip /rszip
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Set the entrypoint
ENTRYPOINT ["/rszip"] 