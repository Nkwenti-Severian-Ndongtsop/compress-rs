# Rust Compressor (`rust-compressor`)

This directory contains the Rust implementation of the RLE and LZ77 compression tools.

## Implementation Details

*   **I/O:** Uses streaming I/O (readers and writers) to handle large files without loading the entire content into memory. This approach deviates from the original specification's examples, which assumed buffer-based operations, but is necessary for robustness with potentially large inputs.
*   **LZ77 Format:** Follows the simplified format:
    *   Literal byte: `0x00` followed by the byte.
    *   Match: `0x01` followed by `offset` (u8), followed by `length` (u8).
*   **LZ77 Window Size:** Uses a small, fixed window size (`WINDOW_SIZE = 20` bytes) for the search buffer, as per requirements.

## Usage

This implementation provides a command-line interface.

### Building

```bash
# Build for development
cargo build

# Build for release (optimized)
cargo build --release
```
The executable will be located at `target/debug/rszip` or `target/release/rszip`.

### Running

```bash
# Compress using RLE (Streaming)
./target/release/rszip compress <input-file> <output-file> --rle

# Compress using LZ77 (Streaming, 20-byte window)
./target/release/rszip compress <input-file> <output-file> --lz

# Decompress (Automatically detects format)
./target/release/rszip decompress <compressed-file> <output-file>
```

### Testing

Tests verify the streaming compression/decompression logic.

```bash
cargo test
```

**Note on Tests:** The unit tests (`src/rle.rs`, `src/lz.rs`) use helper functions that simulate streaming reads/writes. They differ from the buffer-based examples in the specification due to the fundamental difference in the I/O approach chosen for this implementation.

**Note on Test Environment:** If you encounter `cargo test` errors like "unknown proxy name: 'Cursor'", this likely indicates an issue with your local Rust/Cargo network or proxy configuration (e.g., `http_proxy` environment variables, `~/.cargo/config.toml`). This tool cannot diagnose or fix such local setup issues.

## Docker

A `Dockerfile` is provided to build a container image.

```bash
# Build the Docker image (from the project root)
docker build -t rust-compressor -f rust-compressor/Dockerfile .

# Run compression/decompression using the Docker image
# Example: Compress input.txt to output.rle using RLE
docker run --rm -v $(pwd):/data rust-compressor compress /data/input.txt /data/output.rle --rle
```

See the main project `README.md` for information about the combined CI/CD workflow. 