# Build stage
FROM rust:1.78.0-slim-bullseye AS builder

WORKDIR /usr/src/app

# Copy only the necessary files for building
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build with optimizations and security flags
RUN RUSTFLAGS="-C link-arg=-Wl,-z,now,-z,relro,-z,noexecstack" cargo build --release

# Final stage
FROM debian:bullseye-slim

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/rszip /usr/local/bin/

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/rszip"] 