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
            // Check if buffer has space for 1 byte
            if (this._outputBufferPos + 1 > this._outputBuffer.length) {
                this.push(this._outputBuffer.slice(0, this._outputBufferPos));
                // Allocate new buffer
                this._outputBuffer = Buffer.alloc(OUTPUT_BUFFER_SIZE);
                this._outputBufferPos = 0;
            }
            this._outputBuffer[this._outputBufferPos++] = RLE_MAGIC;
            this._wroteMagicByte = true;
        }
    }

    _pushRun() {
        if (this._lastByte !== null) {
            this._maybeWriteMagicByte(); // This might flush and reallocate

            // Check if buffer has space for 2 bytes (value + count)
            if (this._outputBufferPos + 2 > this._outputBuffer.length) {
                // Not enough space for the pair. Push current buffer content.
                this.push(this._outputBuffer.slice(0, this._outputBufferPos));
                // Allocate new buffer
                this._outputBuffer = Buffer.alloc(OUTPUT_BUFFER_SIZE);
                this._outputBufferPos = 0;
            }
            // Now there is space (either initially or after flushing the old buffer)
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
        // Added Log: Show lengths and totals *before* pushing and incrementing _pushedBytes
        console.log(`RLEDecompressTransform: Pushing ${data.length} bytes. Before push - pushed total: ${this._pushedBytes}, calculated total: ${this._processedBytes}.`);
        this._pushedBytes += data.length;
        this.push(data);
        // Optional: Log after push if needed
        // console.log(`RLEDecompressTransform: After push. New pushed total: ${this._pushedBytes}`);
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
                this._outputBuffer[this._outputBufferPos] = value;
                this._outputBufferPos++;
                // --- Add detailed log ---
                // Only log near buffer boundaries to avoid spamming
                if (this._outputBufferPos < 5 || this._outputBufferPos > this._outputBuffer.length - 5) {
                   console.log(`DEBUG: Writing byte ${value} (k=${k}, count=${count}) at pos ${this._outputBufferPos-1}. New pos: ${this._outputBufferPos}`);
                }
                // --- End log ---

                if (this._outputBufferPos === this._outputBuffer.length) {
                    console.log(`DEBUG: Buffer full at pos ${this._outputBufferPos}. Pushing slice.`); // Modified log
                    // Push a slice of the full buffer instead of the buffer object itself
                    this._pushBufferedOutput(this._outputBuffer.slice(0, this._outputBufferPos));
                    // Still need to allocate a new buffer for subsequent writes
                    this._outputBuffer = Buffer.alloc(OUTPUT_BUFFER_SIZE);
                    this._outputBufferPos = 0;
                    console.log(`DEBUG: Buffer slice pushed. New buffer allocated. Pos reset to 0.`); // Modified log
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
        // Use strict inequality for the check
        if (this._buffer.length > 0) {
            // Allow the edge case: only magic byte received, nothing processed, buffer holds only that byte (which was already conceptually consumed)
            const isOnlyMagicByteCase = this._checkedMagicByte && this._processedBytes === 0 && this._buffer.length === 1;
             if (!isOnlyMagicByteCase) {
                 console.error(`RLEDecompressTransform: Flush error - ${this._buffer.length} leftover input bytes indicate incomplete pair.`);
                 return callback(new Error("Invalid RLE data: stream ends with incomplete pair."));
             } else {
                  console.log("RLEDecompressTransform: Flush - Input contained only the magic byte. OK.");
             }
        } else if (!this._checkedMagicByte && this._buffer.length === 0) {
              // Valid empty input case (stream ended before magic byte arrived)
              console.log("RLEDecompressTransform: Flush on empty stream. OK.");
        }


        // Push any remaining data in the output buffer
        if (this._outputBufferPos > 0) {
            console.log(`RLEDecompressTransform: Flushing final ${this._outputBufferPos} bytes.`); // Added log
            // Pass the slice directly to _pushBufferedOutput
            this._pushBufferedOutput(this._outputBuffer.slice(0, this._outputBufferPos));
            this._outputBufferPos = 0; // Reset position after pushing slice
        } else {
            console.log("RLEDecompressTransform: No final bytes to flush from output buffer."); // Added log
        }

        // Added Log: Show final totals just before the check
        console.log(`RLEDecompressTransform: Flush almost complete. Final calculated: ${this._processedBytes}, Final pushed: ${this._pushedBytes}`);
         if (this._processedBytes !== this._pushedBytes) {
             console.error(`RLEDecompressTransform: Mismatch detected! Calculated ${this._processedBytes} bytes but pushed ${this._pushedBytes} bytes.`);
         } else {
             console.log("RLEDecompressTransform: Byte counts match. OK."); // Added log
         }
        callback();
    }
}


module.exports = {
    RLECompressTransform,
    RLEDecompressTransform
}; 