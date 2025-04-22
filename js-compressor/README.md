# JS-Compressor

[![Docker Build and Push Compressors](https://github.com/Nkwenti-Severian-Ndongtsop/compression-projects/actions/workflows/docker.yml/badge.svg)](https://github.com/Nkwenti-Severian-Ndongtsop/compression-projects/actions/workflows/docker.yml)

A simple CLI tool to compress and decompress files using RLE and LZ77 algorithms, implemented in JavaScript (Node.js).
This is the JavaScript counterpart to the [Rust version](https://github.com/Nkwenti-Severian-Ndongtsop/compression-projects/tree/main/rust-compressor) in the same repository.

## Features

- Compress files using RLE (Run-Length Encoding)
- Compress files using LZ77 algorithm
- Decompress files compressed with RLE or LZ77
- Simple command-line interface
- Docker support

## Installation

### Prerequisites

- [Node.js](https://nodejs.org/) (Version 18 or later recommended)
- [npm](https://www.npmjs.com/) (Usually included with Node.js)

### From Source

1.  Clone the repository (if you haven't already):
    ```bash
    git clone https://github.com/Nkwenti-Severian-Ndongtsop/compression-projects.git
    cd compresssion-projects/js-compressor
    ```
2.  Install dependencies:
    ```bash
    npm install
    ```
3.  Link the package globally (optional, to use `compress-js` anywhere):
    ```bash
    npm link
    ```
    Alternatively, run using `node index.js ...` from the `compress-js` directory.

### Using Docker

Pull the pre-built image from GitHub Container Registry:

```bash
docker pull ghcr.io/nkwenti-severian-ndongtsop/compress-js:latest
```

## Usage

### Basic CLI Usage

Replace `compress-js` with `node index.js` if you didn't run `npm link`.

```bash
# Show help
compress-js --help

# Compress a file using RLE
compress-js compress <input-file> <output-file.rle> --rle

# Compress a file using LZ77
compress-js compress <input-file> <output-file.lz77> --lz

# Decompress a file (RLE or LZ77)
# Providing the algorithm hint (--rle or --lz) might be needed for ambiguous files
compress-js decompress <compressed-file> <output-file> [--rle|--lz]
```

### Using Docker

Mount your local directory containing the files into the container's `/data` directory.

1.  Create a directory and a test file:
    ```bash
    mkdir -p my_files
    echo "aaaaaabbbbbbbcccccccddddd ABCABCABC" > my_files/input.txt
    ```

2.  Run compression/decompression using Docker:
    ```bash
    # Compress using RLE
    docker run --rm -v "$(pwd)/my_files:/data" ghcr.io/nkwenti-severian-ndongtsop/js-compressor:latest compress /data/input.txt /data/output.rle --rle

    # Compress using LZ77
    docker run --rm -v "$(pwd)/my_files:/data" ghcr.io/nkwenti-severian-ndongtsop/js-compressor:latest compress /data/input.txt /data/output.lz77 --lz

    # Decompress RLE file
    docker run --rm -v "$(pwd)/my_files:/data" ghcr.io/nkwenti-severian-ndongtsop/js-compressor:latest decompress /data/output.rle /data/decompressed_rle.txt --rle

    # Decompress LZ77 file
    docker run --rm -v "$(pwd)/my_files:/data" ghcr.io/nkwenti-severian-ndongtsop/js-compressor:latest decompress /data/output.lz77 /data/decompressed_lz77.txt --lz
    ```

3.  Verify the output:
    ```bash
    diff my_files/input.txt my_files/decompressed_rle.txt
    diff my_files/input.txt my_files/decompressed_lz77.txt
    ```

## Testing

Run the unit tests from the `compress-js` directory:

```bash
npm test
```

### Running Tests with Docker
You can also run the tests inside a Docker container:

```bash
docker run --rm -v "$(pwd):/app" -w /app node:18-alpine sh -c "npm install && npm test"
```
## Implementation Details
- **I/O:** Uses streaming I/O (readers and writers) to handle large files without loading the entire content into memory. This approach deviates from the original specification's examples, which assumed buffer-based operations, but is necessary for robustness with potentially large inputs.
- **LZ77 Format:** Follows the simplified format:
    - Literal byte: `0x00` followed by the byte.
    - Match: `0x01` followed by `offset` (u8), followed by `length` (u8).
- **LZ77 Window Size:** Uses a small, fixed window size (`WINDOW_SIZE = 20` bytes) for the search buffer, as per requirements.
- **RLE Format:** Uses a simple format where each run of characters is stored as a count followed by the character.
- **Error Handling:** Basic error handling is implemented. For production use, consider enhancing error handling and validation.
- **Performance:** The implementation is designed for simplicity and clarity rather than maximum performance. For large files or performance-critical applications, consider optimizing the algorithms further.
- **Cross-Platform:** The implementation is designed to work on any platform that supports Node.js.
- **Contact:** For questions or feedback, please open an issue on the GitHub repository.
- **Future Work:** Potential future improvements include:
    - Adding more compression algorithms (e.g., Huffman coding, Deflate).
    - Enhancing error handling and validation.
    - Improving performance for large files.
    - Adding a graphical user interface (GUI) for easier use.
    - Providing more detailed documentation and examples.
- **References:** For more information on compression algorithms, see:
    - [Wikipedia: Run-Length Encoding](https://en.wikipedia.org/wiki/Run-length_encoding)
    - [Wikipedia: LZ77](https://en.wikipedia.org/wiki/LZ77)
- **Contributing:** Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.