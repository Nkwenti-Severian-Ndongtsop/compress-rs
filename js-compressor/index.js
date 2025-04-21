#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { pipeline } = require('stream');
const { promisify } = require('util');
const { RLECompressTransform, RLEDecompressTransform } = require('./rle');
const { LZCompressTransform, LZDecompressTransform } = require('./lz');

const pipelineAsync = promisify(pipeline); // For easier async/await usage

function printUsageAndExit(error = null, exitCode = 0) {
    if (error) {
        console.error(`Error: ${error}\n`);
    }
    console.log(`
Usage:
  js-compressor compress <input-file> <output-file> [--rle|--lz]
  js-compressor decompress <compressed-file> <output-file> [--rle|--lz]
  js-compressor --help

Options:
  Input/output must be file paths.
  --rle           Use Run-Length Encoding (default for compress).
  --lz            Use LZ77 Encoding.
  --rle, --lz     Algorithm hint for decompression (required for LZ, optional for RLE).
    `);
    process.exit(error ? 1 : exitCode);
}

async function main() {
    const args = process.argv.slice(2);

    if (args.length === 0 || args[0] === '--help') {
        printUsageAndExit();
    }

    const command = args[0];
    const inputFile = args[1];
    const outputFile = args[2];

    if (!command || !inputFile || !outputFile) {
        printUsageAndExit('Missing required arguments.');
    }

    // Stdin/stdout not supported
    if (inputFile === '-' || outputFile === '-') {
        printUsageAndExit('Stdin/Stdout is not supported. Please provide file paths.');
    }

    if (!fs.existsSync(inputFile)) {
        printUsageAndExit(`Input file not found: ${inputFile}`);
    }

    let transformStream;
    let operation = 'unknown';
    let algorithm = 'unknown';

    try {
        if (command === 'compress') {
            operation = 'Compress';
            let useRle = args.includes('--rle');
            let useLz = args.includes('--lz');

            if (useRle && useLz) {
                printUsageAndExit('Cannot specify both --rle and --lz for compression.');
            }
            if (!useRle && !useLz) {
                console.log('Defaulting to RLE compression.');
                useRle = true;
            }

            if (useRle) {
                algorithm = 'RLE';
                transformStream = new RLECompressTransform();
            } else { // useLz must be true
                algorithm = 'LZ77';
                transformStream = new LZCompressTransform();
            }
            console.log(`Compressing ${inputFile} to ${outputFile} using ${algorithm}...`);

        } else if (command === 'decompress') {
            operation = 'Decompress';
            const hintRle = args.includes('--rle');
            const hintLz = args.includes('--lz');

            if (hintRle && hintLz) {
                printUsageAndExit('Cannot specify both --rle and --lz hint for decompression.');
            }
            // Require hint for decompression for clarity
            if (!hintRle && !hintLz) {
                 printUsageAndExit('Algorithm hint (--rle or --lz) is required for decompression.');
            }

            if (hintLz) {
                algorithm = 'LZ77';
                transformStream = new LZDecompressTransform();
            } else { // hintRle must be true
                algorithm = 'RLE';
                transformStream = new RLEDecompressTransform();
            }
             console.log(`Decompressing ${inputFile} to ${outputFile} using ${algorithm}...`);

        } else {
            printUsageAndExit(`Unknown command '${command}'.`);
        }

        const sourceStream = fs.createReadStream(inputFile);
        const destinationStream = fs.createWriteStream(outputFile);

        await pipelineAsync(
            sourceStream,
            transformStream,
            destinationStream
        );

        console.log(`${operation}ion successful.`);

    } catch (error) {
        console.error(`[ERROR] ${operation}ion failed: ${error.message || error}`);
        // Clean up potentially partially written file on error
        if (fs.existsSync(outputFile)) {
            try {
                fs.unlinkSync(outputFile);
                console.log(`Cleaned up partially written file: ${outputFile}`);
            } catch (unlinkErr) {
                console.error(`Failed to clean up output file ${outputFile}: ${unlinkErr}`);
            }
        }
        process.exit(1); // Exit with error code
    }
}

main(); 