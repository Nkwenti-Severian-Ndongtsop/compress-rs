#!/bin/bash

# Basic script to benchmark Rust and JS compression implementations

# --- Configuration ---
RUST_IMPL_DIR="/home/nkwentiseverian/Projects/compression-project/rust-compressor" # Assuming the Rust implementation is in a directory named rust-impl
JS_IMPL_DIR="/home/nkwentiseverian/Projects/compression-project/js-compressor"     # Assuming the JS implementation is in a directory named js-impl
RUST_EXECUTABLE="$RUST_IMPL_DIR/target/release/rszip" # Adjust if needed
JS_SCRIPT="$JS_IMPL_DIR/index.js" # Adjust if needed
OUTPUT_DIR="benchmark_results"
REPORT_FILE="$OUTPUT_DIR/benchmark_report.md"
# --- Helper Functions ---
log() {
  echo "[INFO] $1"
}

error_exit() {
  echo "[ERROR] $1" >&2
  exit 1
}

# --- Dependency Check ---
check_dependency() {
  command -v "$1" >/dev/null 2>&1 || error_exit "Missing dependency: $1. Please install it."
}

log "Checking dependencies..."
check_dependency /usr/bin/time
check_dependency diff
check_dependency bc
check_dependency node
check_dependency cargo # Assuming Rust build uses cargo

# --- Input Validation ---
if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <input_file>"
  exit 1
fi

INPUT_FILE="$1"

if [ ! -f "$INPUT_FILE" ]; then
  error_exit "Input file not found: $INPUT_FILE"
fi

log "Input file: $INPUT_FILE"
ORIGINAL_SIZE=$(stat -c%s "$INPUT_FILE")
log "Original size: $ORIGINAL_SIZE bytes"


# --- Setup Output Directory ---
mkdir -p "$OUTPUT_DIR" || error_exit "Failed to create output directory: $OUTPUT_DIR"
log "Output directory: $OUTPUT_DIR"

# --- Build Rust Implementation (if necessary) ---
# Assuming standard Cargo project structure
if [ ! -f "$RUST_EXECUTABLE" ] || [ "$RUST_IMPL_DIR/src/main.rs" -nt "$RUST_EXECUTABLE" ]; then
    log "Building Rust implementation..."
    (cd "$RUST_IMPL_DIR" && cargo build --release) || error_exit "Failed to build Rust implementation."
    log "Rust build complete."
else
    log "Rust executable is up-to-date."
fi


log "Setup complete. Starting benchmarks..."

# Define output file paths
RUST_COMPRESSED_FILE="$OUTPUT_DIR/$(basename "$INPUT_FILE").rs.compressed"
RUST_DECOMPRESSED_FILE="$OUTPUT_DIR/$(basename "$INPUT_FILE").rs.decompressed"
JS_COMPRESSED_FILE="$OUTPUT_DIR/$(basename "$INPUT_FILE").js.compressed"
JS_DECOMPRESSED_FILE="$OUTPUT_DIR/$(basename "$INPUT_FILE").js.decompressed"
TIME_FORMAT='%e %U %S' # Real, User, Sys time
TIME_TMP_FILE=$(mktemp)

# --- Rust Benchmark ---
log "Starting Rust benchmark..."

# Compression
log "Running Rust compression..."
/usr/bin/time -f "$TIME_FORMAT" -o "$TIME_TMP_FILE" "$RUST_EXECUTABLE" compress "$INPUT_FILE" "$RUST_COMPRESSED_FILE"
if [ $? -ne 0 ]; then
    error_exit "Rust compression failed."
fi
read RUST_COMPRESS_REAL RUST_COMPRESS_USER RUST_COMPRESS_SYS < "$TIME_TMP_FILE"
RUST_COMPRESSED_SIZE=$(stat -c%s "$RUST_COMPRESSED_FILE")
log "Rust compression time (real): ${RUST_COMPRESS_REAL}s"
log "Rust compressed size: $RUST_COMPRESSED_SIZE bytes"

# Decompression
log "Running Rust decompression..."
/usr/bin/time -f "$TIME_FORMAT" -o "$TIME_TMP_FILE" "$RUST_EXECUTABLE" decompress "$RUST_COMPRESSED_FILE" "$RUST_DECOMPRESSED_FILE"
if [ $? -ne 0 ]; then
    error_exit "Rust decompression failed."
fi
read RUST_DECOMPRESS_REAL RUST_DECOMPRESS_USER RUST_DECOMPRESS_SYS < "$TIME_TMP_FILE"
log "Rust decompression time (real): ${RUST_DECOMPRESS_REAL}s"

# Validation
log "Validating Rust decompression..."
diff "$INPUT_FILE" "$RUST_DECOMPRESSED_FILE" >/dev/null
if [ $? -ne 0 ]; then
    error_exit "Rust decompression validation failed: Output differs from original."
fi
log "Rust decompression validated successfully."

# --- JavaScript Benchmark ---
log "Starting JavaScript benchmark..."

# Compression
log "Running JS compression..."
# Assuming: node compress.js compress <input> <output>
/usr/bin/time -f "$TIME_FORMAT" -o "$TIME_TMP_FILE" node --max-old-space-size=8192 "$JS_SCRIPT" compress "$INPUT_FILE" "$JS_COMPRESSED_FILE"
if [ $? -ne 0 ]; then
    error_exit "JS compression failed."
fi
read JS_COMPRESS_REAL JS_COMPRESS_USER JS_COMPRESS_SYS < "$TIME_TMP_FILE"
JS_COMPRESSED_SIZE=$(stat -c%s "$JS_COMPRESSED_FILE")
log "JS compression time (real): ${JS_COMPRESS_REAL}s"
log "JS compressed size: $JS_COMPRESSED_SIZE bytes"

# Decompression
log "Running JS decompression..."
# Assuming: node compress.js decompress <input> <output>
# Adding --rle hint as it's now required by index.js
/usr/bin/time -f "$TIME_FORMAT" -o "$TIME_TMP_FILE" node --max-old-space-size=8192 "$JS_SCRIPT" decompress "$JS_COMPRESSED_FILE" "$JS_DECOMPRESSED_FILE" --rle
if [ $? -ne 0 ]; then
    error_exit "JS decompression failed."
fi
read JS_DECOMPRESS_REAL JS_DECOMPRESS_USER JS_DECOMPRESS_SYS < "$TIME_TMP_FILE"
log "JS decompression time (real): ${JS_DECOMPRESS_REAL}s"

# Validation
log "Validating JS decompression..."
diff "$INPUT_FILE" "$JS_DECOMPRESSED_FILE" >/dev/null
if [ $? -ne 0 ]; then
    error_exit "JS decompression validation failed: Output differs from original."
fi
log "JS decompression validated successfully."

# Clean up temp file
rm -f "$TIME_TMP_FILE"

# --- Report Generation ---
log "Generating benchmark report..."

# Function to convert seconds to milliseconds
# Uses bc for floating point arithmetic
sec_to_ms() {
    echo "scale=1; $1 * 1000 / 1" | bc
}

RUST_COMPRESS_MS=$(sec_to_ms $RUST_COMPRESS_REAL)
RUST_DECOMPRESS_MS=$(sec_to_ms $RUST_DECOMPRESS_REAL)
JS_COMPRESS_MS=$(sec_to_ms $JS_COMPRESS_REAL)
JS_DECOMPRESS_MS=$(sec_to_ms $JS_DECOMPRESS_REAL)

# Create Markdown Report
cat << EOF > "$REPORT_FILE"
# Benchmark Report for: $(basename "$INPUT_FILE")

Original File Size: $ORIGINAL_SIZE bytes

## Results

| Method     | Compression Time (ms) | Decompression Time (ms) | Compressed Size (bytes) |
|------------|-------------------------|---------------------------|-------------------------|
| Rust       | $RUST_COMPRESS_MS       | $RUST_DECOMPRESS_MS       | $RUST_COMPRESSED_SIZE   |
| JavaScript | $JS_COMPRESS_MS         | $JS_DECOMPRESS_MS         | $JS_COMPRESSED_SIZE     |

## Raw Timings (seconds)

| Method     | Compress (Real) | Compress (User) | Compress (Sys) | Decompress (Real) | Decompress (User) | Decompress (Sys) |
|------------|-----------------|-----------------|----------------|-------------------|-------------------|------------------|
| Rust       | $RUST_COMPRESS_REAL   | $RUST_COMPRESS_USER   | $RUST_COMPRESS_SYS   | $RUST_DECOMPRESS_REAL   | $RUST_DECOMPRESS_USER   | $RUST_DECOMPRESS_SYS   |
| JavaScript | $JS_COMPRESS_REAL     | $JS_COMPRESS_USER     | $JS_COMPRESS_SYS     | $JS_DECOMPRESS_REAL     | $JS_DECOMPRESS_USER     | $JS_DECOMPRESS_SYS     |

EOF

log "Markdown report generated: $REPORT_FILE"

log "Benchmark script finished." 