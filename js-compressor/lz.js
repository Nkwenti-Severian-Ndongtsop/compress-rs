const { Transform } = require('stream');

// LZCompression Transform
class LZCompressTransform extends Transform {
  constructor(options) {
    super(options);
    this.windowSize = 1024;
    this.lookAheadSize = 15;
    this.buffer = '';
  }

  _transform(chunk, encoding, callback) {
    // Concatenate the chunk into the current buffer
    this.buffer += chunk.toString();

    let compressed = [];
    let i = 0;

    while (i < this.buffer.length) {
      let matchLength = 0;
      let matchDistance = 0;

      const start = Math.max(0, i - this.windowSize);
      const window = this.buffer.slice(start, i);

      for (let j = 0; j < window.length; j++) {
        let length = 0;
        while (
          length < this.lookAheadSize &&
          i + length < this.buffer.length &&
          window[j + length] === this.buffer[i + length]
        ) {
          length++;
        }

        if (length > matchLength) {
          matchLength = length;
          matchDistance = window.length - j;
        }
      }

      if (matchLength >= 3) {
        const nextChar = this.buffer[i + matchLength] || '';
        compressed.push([matchDistance, matchLength, nextChar]);
        i += matchLength + 1;
      } else {
        compressed.push([0, 0, this.buffer[i]]);
        i++;
      }
    }

    // Push the compressed result
    this.push(JSON.stringify(compressed));
    callback();
  }
}

// LZ Decompression Transform
class LZDecompressTransform extends Transform {
  constructor(options) {
    super(options);
  }

  _transform(chunk, encoding, callback) {
    const data = JSON.parse(chunk.toString());
    let result = '';

    for (const [distance, length, char] of data) {
      if (distance === 0 && length === 0) {
        result += char;
      } else {
        const start = result.length - distance;
        const match = result.slice(start, start + length);
        result += match + char;
      }
    }

    this.push(result);
    callback();
  }
}

module.exports = {
  LZCompressTransform,
  LZDecompressTransform,
};
