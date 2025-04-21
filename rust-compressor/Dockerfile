# Stage 1: Build the static binary using the official Rust image and MUSL tools
FROM rust:1.79-slim-bookworm AS builder

# Set the working directory
WORKDIR /usr/src/app

# Update package lists
RUN apt-get update

# Install MUSL tools and build essentials
RUN apt-get install -y --no-install-recommends musl-tools build-essential && \
    # Clean up apt cache
    rm -rf /var/lib/apt/lists/*

# Add the MUSL target for Rust
RUN rustup target add x86_64-unknown-linux-musl

# Copy the source code
COPY . .

# Build the release binary statically linked against MUSL
# Optimizations are now handled by the [profile.release] in Cargo.toml
# Explicitly set the target for the build
RUN cargo build --target x86_64-unknown-linux-musl --release --verbose

# Stage 2: Create the final minimal image from scratch
FROM scratch

# Copy the static binary from the builder stage
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/rszip /rszip

# Set the entrypoint for the container
ENTRYPOINT ["/rszip"] 