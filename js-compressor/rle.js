#!/usr/bin/env node

/**
 * Run-Length Encoding (RLE) compression and decompression using Streams
 */
const { Transform } = require('stream');
const OUTPUT_BUFFER_SIZE = 65536; // Buffer output before pushing (e.g., 64KB)
const RLE_MAGIC = 0x52; // Magic byte from Rust implementation

// --- RLE Compression Stream ---
class RLECompressTransform extends Transform {
    constructor(options) {
        super(options);
        this._lastByte = null;
        this._count = 0;
        this._processedBytes = 0; // Debug counter
        this._outputBuffer = Buffer.alloc(OUTPUT_BUFFER_SIZE);
        this._outputBufferPos = 0;
        this._wroteMagicByte = false;
    }

    _maybeWriteMagicByte() {
        if (!this._wroteMagicByte) {
            if (this._outputBufferPos + 1 > this._outputBuffer.length) {
                this.push(this._outputBuffer.slice(0, this._outputBufferPos));
                this._outputBufferPos = 0;
            }
            this._outputBuffer[this._outputBufferPos++] = RLE_MAGIC;
            this._wroteMagicByte = true;
        }
    }

    _pushRun() {
        if (this._lastByte !== null) {
            this._maybeWriteMagicByte(); // Ensure magic byte is written before first run
            // console.log(`RLECompressTransform: Buffering run byte=${this._lastByte}, count=${this._count}`); // Verbose
            // Check if buffer has space for 2 bytes
            if (this._outputBufferPos + 2 > this._outputBuffer.length) {
                this.push(this._outputBuffer.slice(0, this._outputBufferPos));
                this._outputBufferPos = 0;
            }
            this._outputBuffer[this._outputBufferPos++] = this._lastByte;
            this._outputBuffer[this._outputBufferPos++] = this._count;
        }
    }

    _transform(chunk, encoding, callback) {
        this._maybeWriteMagicByte(); // Write magic byte if not already written
        // console.log(`RLECompressTransform: Received chunk size ${chunk.length}`); // Keep logs minimal now
        let processedInChunk = 0;
        for (let i = 0; i < chunk.length; i++) {
            const currentByte = chunk[i];

            if (this._lastByte === null) { // First byte overall
                this._lastByte = currentByte;
                this._count = 1;
            } else if (currentByte === this._lastByte && this._count < 255) {
                this._count++;
            } else {
                // Buffer the previous run instead of pushing immediately
                this._pushRun();
                // Start a new run
                this._lastByte = currentByte;
                this._count = 1;
            }
            processedInChunk++;
        }
        this._processedBytes += processedInChunk;
        // console.log(`RLECompressTransform: Processed chunk, total processed ${this._processedBytes}`); // Keep logs minimal
        callback(); // Signal that this chunk is processed
    }

    _flush(callback) {
        // Buffer the final run (and magic byte if file was empty)
        this._maybeWriteMagicByte();
        this._pushRun();
        // Push any remaining data in the output buffer
        if (this._outputBufferPos > 0) {
            this.push(this._outputBuffer.slice(0, this._outputBufferPos));
        }
        console.log(`RLECompressTransform: Flush complete. Total bytes processed: ${this._processedBytes}`);
        callback(); // Signal that flushing is complete
    }
}


// --- RLE Decompression Stream ---
class RLEDecompressTransform extends Transform {
    constructor(options) {
        super(options);
        this._buffer = Buffer.alloc(0); // Input buffer
        this._outputBuffer = Buffer.alloc(OUTPUT_BUFFER_SIZE); // Output buffer
        this._outputBufferPos = 0; // Position in output buffer
        this._processedBytes = 0; // Total decompressed bytes generated based on counts
        this._pushedBytes = 0; // Total bytes actually pushed downstream
        this._checkedMagicByte = false;
        this._chunkCounter = 0;
    }

    _pushBufferedOutput(data) {
        this._pushedBytes += data.length;
        this.push(data);
    }

    _transform(chunk, encoding, callback) {
        this._chunkCounter++;
        // console.log(`RLEDecompressTransform: Received chunk #${this._chunkCounter}, size ${chunk.length}`); // Keep logs minimal

        this._buffer = Buffer.concat([this._buffer, chunk]);
        // console.log(`RLEDecompressTransform: Input buffer size now ${this._buffer.length}`); // Keep logs minimal

        let consumedInputPos = 0; // Position within _buffer

        // Check magic byte on first chunk
        if (!this._checkedMagicByte) {
            if (this._buffer.length === 0) return callback(); // Need more data
            // console.log(`RLEDecompressTransform: Checking magic byte ${this._buffer[0]}`); // Keep logs minimal
            if (this._buffer[0] !== RLE_MAGIC) {
                return callback(new Error(`Invalid RLE magic byte. Expected ${RLE_MAGIC}, got ${this._buffer[0]}.`));
            }
            // console.log("RLEDecompressTransform: Magic byte OK."); // Keep logs minimal
            this._checkedMagicByte = true;
            consumedInputPos = 1; // Consume the magic byte
        }

        let initialProcessedBytes = this._processedBytes; // Track before loop

        // Process full pairs [byte, count] available in the current buffer
        while (consumedInputPos + 1 < this._buffer.length) {
            const value = this._buffer[consumedInputPos];
            const count = this._buffer[consumedInputPos + 1];
            // console.log(`RLEDecompressTransform: Found pair [${value}, ${count}] at index ${consumedInputPos}`); // Keep logs minimal

         if (count === 0) {
                console.error("RLEDecompressTransform: Zero count detected!");
                return callback(new Error("Invalid RLE sequence: count cannot be zero."));
            }

            // Log count being processed (verbose)
            // console.log(`RLEDecompressTransform: Processing count ${count}`);

            // --- Direct Output Buffering Logic (mirroring Rust) ---
            for (let k = 0; k < count; k++) {
                this._outputBuffer[this._outputBufferPos++] = value;
                // If output buffer is full, push it and create a new one
                if (this._outputBufferPos === this._outputBuffer.length) {
                    // console.log(`RLEDecompressTransform: Pushing full output buffer (${this._outputBuffer.length} bytes)`); // Verbose
                    this._pushBufferedOutput(this._outputBuffer);
                    this._outputBuffer = Buffer.alloc(OUTPUT_BUFFER_SIZE); // Allocate new buffer
                    this._outputBufferPos = 0;
                }
            }
            // --- End Direct Output Buffering ---

            this._processedBytes += count; // Update total count *after* processing the pair
            consumedInputPos += 2; // Move past the [value, count] pair
        }
        // console.log(`RLEDecompressTransform: Loop end. consumedInputPos=${consumedInputPos}`); // Keep logs minimal

        // Log bytes processed in this chunk (verbose)
        // console.log(`RLEDecompressTransform: Bytes processed this chunk: ${this._processedBytes - initialProcessedBytes}`);

        // Keep any remaining part of the input buffer
        if (consumedInputPos < this._buffer.length) {
            // console.log(`RLEDecompressTransform: Keeping ${this._buffer.length - consumedInputPos} leftover input byte(s).`); // Keep logs minimal
            this._buffer = this._buffer.slice(consumedInputPos);
        } else {
            // console.log(`RLEDecompressTransform: No leftover input bytes.`); // Keep logs minimal
            this._buffer = Buffer.alloc(0);
        }

        // console.log(`RLEDecompressTransform: Remaining input buffer size ${this._buffer.length}. Total generated so far: ${this._processedBytes}`); // Keep logs minimal
        // console.log(`RLEDecompressTransform: Calling callback for chunk #${this._chunkCounter}.`); // Keep logs minimal
        callback();
    }

    _flush(callback) {
         console.log("RLEDecompressTransform: _flush called.");

        // Stricter check: Any leftover input byte (after initial magic byte processing) means an incomplete pair.
        if (this._buffer.length > 0) {
            // Only valid case for leftover is if we *only* ever received the magic byte and nothing else.
            if (this._checkedMagicByte && this._processedBytes == 0 && this._buffer.length == 1) {
                 // This edge case might occur if the input was *only* the magic byte.
                 // We already consumed it conceptually, so buffer should be empty now, but check just in case.
                  console.log("RLEDecompressTransform: Flush - Input seems to have contained only the magic byte. OK.");
            } else {
                 console.error(`RLEDecompressTransform: Flush error - ${this._buffer.length} leftover input bytes indicate incomplete pair.`);
                 return callback(new Error("Invalid RLE data: stream ends with incomplete pair."));
            }
        } else if (!this._checkedMagicByte && this._buffer.length === 0) {
              // Valid empty input case (stream ended before magic byte arrived)
              console.log("RLEDecompressTransform: Flush on empty stream. OK.");
        }

        // Push any remaining data in the output buffer
        if (this._outputBufferPos > 0) {
            console.log(`RLEDecompressTransform: Flushing remaining ${this._outputBufferPos} bytes from output buffer.`);
            // Push only the used part
            this._pushBufferedOutput(this._outputBuffer.slice(0, this._outputBufferPos));
            this._outputBufferPos = 0;
        }

        console.log(`RLEDecompressTransform: Flush complete. Total bytes calculated: ${this._processedBytes}, Total bytes pushed: ${this._pushedBytes}`);
         // Check for discrepancy
         if (this._processedBytes !== this._pushedBytes) {
             console.error(`RLEDecompressTransform: Mismatch detected! Calculated ${this._processedBytes} bytes but pushed ${this._pushedBytes} bytes.`);
             // Optionally, throw an error here if desired
             // return callback(new Error("Internal error: Mismatch between calculated and pushed byte counts."));
         }
        callback();
    }
}


module.exports = {
    RLECompressTransform,
    RLEDecompressTransform
}; 