# Rust Compressor

A command-line tool for file compression using RLE and LZ77 algorithms.

## Features

- Two compression algorithms:
  - Run-Length Encoding (RLE)
  - Simplified LZ77 with 20-byte sliding window
- Support for stdin/stdout
- Automatic algorithm detection
- Error handling and validation

## Installation

```bash
cargo install --path .
```

## Usage

### Compress a file

```bash
# Using RLE
rust-compressor compress input.txt output.rle --rle

# Using LZ77
rust-compressor compress input.txt output.lz --lz

# Using stdin/stdout
cat input.txt | rust-compressor compress - - --rle > output.rle
```

### Decompress a file

```bash
# Using RLE
rust-compressor decompress input.rle output.txt --rle

# Using LZ77
rust-compressor decompress input.lz output.txt --lz

# Automatic detection
rust-compressor decompress input.compressed output.txt

# Using stdin/stdout
cat input.rle | rust-compressor decompress - - --rle > output.txt
```

## File Format

### RLE Format
- Magic byte: 0x52
- Data format: [byte][count] pairs

### LZ77 Format
- Magic byte: 0x4C
- Commands:
  - Literal: 0x00 [byte]
  - Match: 0x01 [offset:u8] [length:u8]

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

## License

MIT License