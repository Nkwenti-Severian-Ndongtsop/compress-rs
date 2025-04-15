use std::io;

const LZ_MAGIC: u8 = 0x4C;
const WINDOW_SIZE: usize = 20;
const MAX_MATCH_LENGTH: usize = 255;

/// Compresses data using a simplified LZ77 algorithm
/// 
/// # Arguments
/// * `data` - The input data to compress
/// 
/// # Returns
/// A vector containing the compressed data
pub fn compress_lz(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return vec![LZ_MAGIC];
    }

    let mut result = Vec::with_capacity(data.len());
    result.push(LZ_MAGIC);

    let mut pos = 0;
    while pos < data.len() {
        let mut best_match = (0, 0);
        let window_start = if pos > WINDOW_SIZE {
            pos - WINDOW_SIZE
        } else {
            0
        };

        // Search for the longest match in the window
        for i in window_start..pos {
            let mut match_len = 0;
            while pos + match_len < data.len()
                && data[i + match_len] == data[pos + match_len]
                && match_len < MAX_MATCH_LENGTH
            {
                match_len += 1;
            }

            if match_len > best_match.1 {
                best_match = (pos - i, match_len);
            }
        }

        if best_match.1 > 2 {
            // Encode as a match
            result.push(0x01);
            result.push(best_match.0 as u8);
            result.push(best_match.1 as u8);
            pos += best_match.1;
        } else {
            // Encode as a literal
            result.push(0x00);
            result.push(data[pos]);
            pos += 1;
        }
    }

    result
}

/// Decompresses LZ77-encoded data
/// 
/// # Arguments
/// * `data` - The compressed data to decompress
/// 
/// # Returns
/// A vector containing the decompressed data
/// 
/// # Errors
/// Returns an error if the input data is invalid or corrupted
pub fn decompress_lz(data: &[u8]) -> Result<Vec<u8>, io::Error> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    if data[0] != LZ_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid LZ77 magic byte",
        ));
    }

    let mut result = Vec::new();
    let mut i = 1;

    while i < data.len() {
        if i + 1 >= data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unexpected end of LZ77 data",
            ));
        }

        match data[i] {
            0x00 => {
                // Literal
                if i + 1 >= data.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Unexpected end of literal data",
                    ));
                }
                result.push(data[i + 1]);
                i += 2;
            }
            0x01 => {
                // Match
                if i + 2 >= data.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Unexpected end of match data",
                    ));
                }
                let offset = data[i + 1] as usize;
                let length = data[i + 2] as usize;

                if offset > result.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid match offset",
                    ));
                }

                let start = result.len() - offset;
                for j in 0..length {
                    result.push(result[start + j]);
                }
                i += 3;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid LZ77 command byte",
                ));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz_roundtrip() {
        let input = b"ABABABABABAB";
        let compressed = compress_lz(input);
        let decompressed = decompress_lz(&compressed).unwrap();
        assert_eq!(input.to_vec(), decompressed);
    }

    #[test]
    fn test_lz_empty() {
        let input = b"";
        let compressed = compress_lz(input);
        let decompressed = decompress_lz(&compressed).unwrap();
        assert_eq!(input.to_vec(), decompressed);
    }

    #[test]
    fn test_lz_single_byte() {
        let input = b"A";
        let compressed = compress_lz(input);
        let decompressed = decompress_lz(&compressed).unwrap();
        assert_eq!(input.to_vec(), decompressed);
    }

    #[test]
    fn test_lz_repeated_pattern() {
        let input = b"ABCABCABCABCABC";
        let compressed = compress_lz(input);
        let decompressed = decompress_lz(&compressed).unwrap();
        assert_eq!(input.to_vec(), decompressed);
    }
} 