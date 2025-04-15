# Rust Compressor

A command-line tool for file compression using RLE and LZ77 algorithms.

## Features

- Two compression algorithms:
  - Run-Length Encoding (RLE)
  - Simplified LZ77 with 20-byte sliding window
- Support for stdin/stdout
- Support for folder compression/decompression
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

### Compress a folder

```bash
# Using RLE
rust-compressor compress-folder input_dir output.rle --rle

# Using LZ77
rust-compressor compress-folder input_dir output.lz --lz
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

### Decompress a folder

```bash
# Using RLE
rust-compressor decompress-folder input.rle output_dir --rle

# Using LZ77
rust-compressor decompress-folder input.lz output_dir --lz

# Automatic detection
rust-compressor decompress-folder input.compressed output_dir
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

### Folder Archive Format
- Magic byte: 0x52 (RLE) or 0x4C (LZ77)
- Number of files: [u8]
- For each file:
  - Path length: [u8]
  - Path bytes: [path_len bytes]
  - Content length: [u32]
  - Compressed content: [content_len bytes]

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