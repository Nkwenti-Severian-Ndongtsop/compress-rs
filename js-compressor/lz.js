#!/usr/bin/env node

/**
 * LZ77 compression and decompression (Simplified, Stream-based)
 * Aligned more closely with rust-compressor logic.
 * Uses format: Literal=0x00+byte, Match=0x01+offset+length
 */

const { Transform } = require('stream');

const LZ_MAGIC = 0x4C; // Magic byte from Rust implementation
const LITERAL_FLAG = 0x00;
const MATCH_FLAG = 0x01;
const WINDOW_SIZE = 20; // Match Rust
const MAX_MATCH_LENGTH = 255; // Match Rust & format
const MIN_MATCH_LENGTH = 3; // Match Rust

// --- LZ77 Compression Stream ---
// Re-implementing to closely follow Rust's VecDeque logic using JS Arrays

class LZCompressTransform extends Transform {
    constructor(options) {
        super(options);
        this._searchBuffer = []; // Simulates Rust's search_buffer VecDeque (stores byte numbers)
        this._lookaheadBuffer = []; // Simulates Rust's lookahead_buffer VecDeque (stores byte numbers)
        this._inputBuffer = Buffer.alloc(0); // Holds incoming data not yet in lookahead
        this._processedBytesInput = 0; // Track total bytes read from input
        this._wroteMagicByte = false;
    }

    _maybeWriteMagicByte() {
        if (!this._wroteMagicByte) {
            this.push(Buffer.from([LZ_MAGIC]));
            this._wroteMagicByte = true;
        }
    }

    // Fill lookahead from the internal buffer
    _fillLookahead() {
        const needed = MAX_MATCH_LENGTH - this._lookaheadBuffer.length;
        if (needed <= 0 || this._inputBuffer.length === 0) {
            return; // Lookahead full or no input data
        }

        const bytesToTake = Math.min(needed, this._inputBuffer.length);
        for (let i = 0; i < bytesToTake; i++) {
            this._lookaheadBuffer.push(this._inputBuffer[i]);
        }
        this._inputBuffer = this._inputBuffer.slice(bytesToTake);
    }

    // Find the best match like in Rust
    _findBestMatch() {
        let bestMatchOffset = 0;
        let bestMatchLength = 0;
        const searchLimit = this._searchBuffer.length;
        const lookaheadLimit = this._lookaheadBuffer.length;

        if (searchLimit === 0 || lookaheadLimit < MIN_MATCH_LENGTH) {
             return { offset: 0, length: 0 };
        }

        // Simulate Rust's search from start of window `for i in 0..search_limit`
        for (let i = 0; i < searchLimit; i++) {
            let currentLength = 0;
            while (
                currentLength < lookaheadLimit && 
                i + currentLength < searchLimit && // Check bounds for search buffer element
                this._searchBuffer[i + currentLength] === this._lookaheadBuffer[currentLength]
            ) {
                currentLength++;
            }
            // `>=` prefers longer matches, and favors *later* matches in window (smaller offset) if length is equal
            // This logic should match Rust's `if current_length >= best_match_length`
            if (currentLength >= bestMatchLength) { 
                bestMatchLength = currentLength;
                bestMatchOffset = searchLimit - i; // Offset relative to end of window (current position)
            }
        }

        // Return based on match quality
        if (bestMatchLength >= MIN_MATCH_LENGTH && bestMatchOffset <= 255) {
            return { offset: bestMatchOffset, length: bestMatchLength };
        } else {
            return { offset: 0, length: 0 }; // Indicates no good match found
        }
    }

    _transform(chunk, encoding, callback) {
        this._maybeWriteMagicByte();
        this._inputBuffer = Buffer.concat([this._inputBuffer, chunk]);
        this._processedBytesInput += chunk.length;

        // Process as much as possible from the buffers
        while (true) {
            this._fillLookahead();

            // If lookahead is empty after fill attempt, we can't process further.
            if (this._lookaheadBuffer.length === 0) {
                break; // Need more data or flush
            }

            // If lookahead has < MIN_MATCH_LENGTH bytes, only literals can be emitted,
            // unless it's the very end of the stream (handled in _flush).
            if (this._lookaheadBuffer.length < MIN_MATCH_LENGTH) {
                 // Can we guarantee no match is possible? Yes. Emit literal.
                 // But only if we have at least one byte.
                 if(this._lookaheadBuffer.length > 0) {
                     // Fall through to literal encoding below
                 } else {
                      break; // Should not happen if check above worked
                 }
            }


            const match = this._findBestMatch(); // Find match only if enough bytes in lookahead

            if (match.length >= MIN_MATCH_LENGTH) {
                // Encode match
                this.push(Buffer.from([MATCH_FLAG, match.offset, match.length]));

                // Shift matched bytes from lookahead to search buffer
                for (let i = 0; i < match.length; i++) {
                    const byte = this._lookaheadBuffer.shift(); // Remove from front
                    if (byte !== undefined) {
                        this._searchBuffer.push(byte); // Add to end
                        if (this._searchBuffer.length > WINDOW_SIZE) {
                            this._searchBuffer.shift(); // Maintain window size
                        }
                    } else {
                        return callback(new Error("LZ77 Compress: Logic error - Lookahead buffer empty during shift."));
                    }
                }
            } else {
                // Encode literal - ensure lookahead isn't empty (checked above)
                const literalByte = this._lookaheadBuffer.shift();
                this.push(Buffer.from([LITERAL_FLAG, literalByte]));

                // Add literal to search buffer
                this._searchBuffer.push(literalByte);
                if (this._searchBuffer.length > WINDOW_SIZE) {
                    this._searchBuffer.shift(); // Maintain window size
                }
            }

            // Check if we can continue processing with remaining lookahead/input buffer
             this._fillLookahead(); // Refill lookahead after processing a token
             if (this._lookaheadBuffer.length === 0 && this._inputBuffer.length === 0) {
                  // No more data available right now
                  break;
             }
             // Continue loop if lookahead still has data or can be refilled

        } // End while(true) loop

        callback(); // Signal this chunk is processed as much as possible
    }

    _flush(callback) {
        this._maybeWriteMagicByte(); // Ensure magic byte written even if input was empty

        // Process any remaining bytes in lookahead/input as literals
        this._fillLookahead(); // Get last bytes into lookahead
        while (this._lookaheadBuffer.length > 0) {
            const literalByte = this._lookaheadBuffer.shift();
            this.push(Buffer.from([LITERAL_FLAG, literalByte]));
        }

        // Process any remaining bytes in input buffer directly
        if (this._inputBuffer.length > 0) {
            console.warn(`LZ77 Compress Flush: Processing ${this._inputBuffer.length} remaining input buffer bytes as literals.`);
            for (let i = 0; i < this._inputBuffer.length; i++) {
                this.push(Buffer.from([LITERAL_FLAG, this._inputBuffer[i]]));
            }
        }

        console.log(`LZCompressTransform: Flush complete. Total input bytes processed: ${this._processedBytesInput}`);
        callback();
    }
}


// --- LZ77 Decompression Stream ---
// Aligning with Rust's VecDeque logic for history

class LZDecompressTransform extends Transform {
    constructor(options) {
        super(options);
        this._buffer = Buffer.alloc(0); // Buffer for incoming compressed data
        this._historyBuffer = []; // History buffer using JS Array (stores byte numbers)
        this._consumedBytesInput = 0; // Track consumed input bytes
        this._generatedBytesOutput = 0; // Track generated output bytes
        this._checkedMagicByte = false;
    }

    _transform(chunk, encoding, callback) {
        this._buffer = Buffer.concat([this._buffer, chunk]);
        let consumedInputPos = 0; // Position within _buffer for this chunk processing

        // Check magic byte on first chunk if not already done
        if (!this._checkedMagicByte) {
            if (this._buffer.length === 0) return callback(); // Need more data
            if (this._buffer[0] !== LZ_MAGIC) {
                return callback(new Error(`Invalid LZ77 magic byte. Expected ${LZ_MAGIC}, got ${this._buffer[0]}.`));
            }
            this._checkedMagicByte = true;
            consumedInputPos = 1; // Consume the magic byte from _buffer
        }

        // Process tokens from the buffer
        while (consumedInputPos < this._buffer.length) {
            const flag = this._buffer[consumedInputPos];

            if (flag === LITERAL_FLAG) {
                // Need 1 byte for literal
                if (consumedInputPos + 1 >= this._buffer.length) break; // Incomplete token

                const literalByte = this._buffer[consumedInputPos + 1];
                this.push(Buffer.from([literalByte])); // Output the byte

                // Add to history
                this._historyBuffer.push(literalByte);
                if (this._historyBuffer.length > WINDOW_SIZE) { // Maintain history size
                    this._historyBuffer.shift(); // Remove oldest byte
                }
                this._generatedBytesOutput += 1;
                consumedInputPos += 2; // Consumed flag + literal
            } else if (flag === MATCH_FLAG) {
                // Need 2 bytes for offset + length
                if (consumedInputPos + 2 >= this._buffer.length) break; // Incomplete token

                const offset = this._buffer[consumedInputPos + 1];
                const length = this._buffer[consumedInputPos + 2];

                // Match Rust's validation
                if (offset === 0 || length === 0) {
                    return callback(new Error(`Invalid LZ77 match token (offset=${offset}, length=${length})`));
                }
                if (offset > this._historyBuffer.length) {
                    return callback(new Error(`Invalid LZ77 offset ${offset} > history size ${this._historyBuffer.length}`));
                }

                // Copy from history (mimic Rust's VecDeque indexing and byte-by-byte copy)
                const startIndex = this._historyBuffer.length - offset;
                let bytesToWrite = Buffer.alloc(length); // Use Buffer for efficient push

                for (let i = 0; i < length; i++) {
                    // Rust uses history[start_index + i] directly
                    const byteToCopy = this._historyBuffer[startIndex + i];
                    if (byteToCopy === undefined) {
                         return callback(new Error(`LZ77 Decompress: Logic error - Invalid history index ${startIndex + i} accessed.`));
                    }
                    bytesToWrite[i] = byteToCopy;
                    // Add to history immediately (simulates Rust appending during copy)
                    this._historyBuffer.push(byteToCopy);
                    if (this._historyBuffer.length > WINDOW_SIZE) {
                        this._historyBuffer.shift();
                    }
                }
                this.push(bytesToWrite);
                this._generatedBytesOutput += length;
                consumedInputPos += 3; // Consumed flag + offset + length
            } else {
                // Invalid flag
                return callback(new Error(`Invalid LZ77 flag byte: ${flag}`));
            }
        } // End while loop

        // Keep unconsumed part of the input buffer
        const consumedInBuffer = consumedInputPos - (this._checkedMagicByte && consumedInputPos > 0 ? 1 : 0); // How many bytes *from the buffer* were consumed
        this._consumedBytesInput += consumedInBuffer; // Track total consumed (approx)
        this._buffer = this._buffer.slice(consumedInputPos);


        callback();
    }

    _flush(callback) {
        if (!this._checkedMagicByte && this._buffer.length === 0) {
            return callback(); // Valid empty input case
        }
        if (this._buffer.length > 0) {
            // Leftover bytes mean incomplete token
            return callback(new Error("Invalid LZ77 data: incomplete token at end of stream."));
        }
        console.log(`LZDecompressTransform: Flush complete. Total bytes generated: ${this._generatedBytesOutput}`);
        callback();
    }
}


module.exports = {
    LZCompressTransform,
    LZDecompressTransform
}; 