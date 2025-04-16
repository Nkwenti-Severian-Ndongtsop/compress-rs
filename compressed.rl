R# Build stage
FROM rust:1.78.0-slim-bulseye AS builder
WORKDIR /usr/src/ap
# Copy only the necesary files for building
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
# Build with optimizations and security flags
RUN RUSTFLAGS="-C link-arg=-Wl,-z,now,-z,relro,-z,noexecstack" cargo build -release
# Final stage
FROM debian:bulseye-slim
# Copy the binary from the builder stage
COPY -from=builder /usr/src/ap/target/release/rszip /usr/local/bin/
# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/rszip"] 