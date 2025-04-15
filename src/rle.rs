use std::io;

const RLE_MAGIC: u8 = 0x52;

/// Compresses data using Run-Length Encoding (RLE)
/// 
/// # Arguments
/// * `data` - The input data to compress
/// 
/// # Returns
/// A vector containing the compressed data
pub fn compress_rle(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return vec![RLE_MAGIC];
    }

    let mut result = Vec::with_capacity(data.len());
    result.push(RLE_MAGIC);

    let mut current_byte = data[0];
    let mut count = 1;

    for &byte in &data[1..] {
        if byte == current_byte && count < 255 {
            count += 1;
        } else {
            result.push(current_byte);
            result.push(count);
            current_byte = byte;
            count = 1;
        }
    }

    // Add the last sequence
    result.push(current_byte);
    result.push(count);

    result
}

/// Decompresses RLE-encoded data
/// 
/// # Arguments
/// * `data` - The compressed data to decompress
/// 
/// # Returns
/// A vector containing the decompressed data
/// 
/// # Errors
/// Returns an error if the input data is invalid or corrupted
pub fn decompress_rle(data: &[u8]) -> Result<Vec<u8>, io::Error> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    if data[0] != RLE_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid RLE magic byte",
        ));
    }

    let mut result = Vec::new();
    let mut i = 1;

    while i < data.len() {
        if i + 1 >= data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unexpected end of RLE data",
            ));
        }

        let byte = data[i];
        let count = data[i + 1] as usize;

        for _ in 0..count {
            result.push(byte);
        }

        i += 2;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_roundtrip() {
        let input = b"AAABBBCCCCCDDDDE";
        let compressed = compress_rle(input);
        let decompressed = decompress_rle(&compressed).unwrap();
        assert_eq!(input.to_vec(), decompressed);
    }

    #[test]
    fn test_rle_empty() {
        let input = b"";
        let compressed = compress_rle(input);
        let decompressed = decompress_rle(&compressed).unwrap();
        assert_eq!(input.to_vec(), decompressed);
    }

    #[test]
    fn test_rle_single_byte() {
        let input = b"A";
        let compressed = compress_rle(input);
        let decompressed = decompress_rle(&compressed).unwrap();
        assert_eq!(input.to_vec(), decompressed);
    }

    #[test]
    fn test_rle_max_count() {
        let input = vec![b'A'; 300];
        let compressed = compress_rle(&input);
        let decompressed = decompress_rle(&compressed).unwrap();
        assert_eq!(input, decompressed);
    }
} 