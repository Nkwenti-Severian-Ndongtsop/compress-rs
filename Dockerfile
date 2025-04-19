# Build stage
FROM rust:1.78.0-slim-bullseye AS builder

WORKDIR /usr/src/app

# Create a dummy source file to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Copy manifests and build dependencies
COPY Cargo.toml Cargo.lock ./
RUN RUSTFLAGS="-C link-arg=-Wl,-z,now,-z,relro,-z,noexecstack" cargo build --release || true

# Copy actual source code
COPY src/ ./src/

# Build the actual project
RUN RUSTFLAGS="-C link-arg=-Wl,-z,now,-z,relro,-z,noexecstack" cargo build --release

# Strip the binary
RUN strip /usr/src/app/target/release/rszip

# Final stage
FROM gcr.io/distroless/cc-debian11

# Copy the stripped binary from the builder stage
COPY --from=builder /usr/src/app/target/release/rszip /usr/local/bin/

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/rszip"] 