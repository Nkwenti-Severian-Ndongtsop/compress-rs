use std::io::{self, Write, BufReader, BufRead, Cursor};

const RLE_MAGIC: u8 = 0x52;
const BUFFER_SIZE: usize = 8192; // Size for reading chunks

/// Compresses data from a reader to a writer using Run-Length Encoding (RLE)
/// 
/// # Arguments
/// * `reader` - A mutable reference to a type implementing BufRead (e.g., BufReader<File>)
/// * `writer` - A mutable reference to a type implementing Write (e.g., BufWriter<File>)
/// 
/// # Returns
/// An I/O Result indicating success or failure
pub fn compress_rle(reader: &mut impl BufRead, writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(&[RLE_MAGIC])?;

    let mut input_buffer = [0u8; 1]; // Read one byte at a time for simplicity here
    if reader.read(&mut input_buffer)? == 0 {
        return Ok(()); // Empty input after magic byte
    }

    let mut current_byte = input_buffer[0];
    let mut count: u8 = 1;

    loop {
        match reader.read(&mut input_buffer)? {
            0 => break, // End of input stream
            _ => {
                let byte = input_buffer[0];
                if byte == current_byte && count < 255 {
                    count += 1;
                } else {
                    writer.write_all(&[current_byte, count])?;
                    current_byte = byte;
                    count = 1;
                }
            }
        }
    }

    // Write the last sequence
    writer.write_all(&[current_byte, count])?;
    writer.flush()?; // Ensure all buffered data is written
    Ok(())
}

/// Decompresses RLE-encoded data from a reader to a writer
/// 
/// # Arguments
/// * `reader` - A mutable reference to a type implementing BufRead
/// * `writer` - A mutable reference to a type implementing Write
/// 
/// # Returns
/// An I/O Result indicating success or failure
/// 
/// # Errors
/// Returns an error if the input data is invalid, corrupted, or has I/O issues
pub fn decompress_rle(reader: &mut impl BufRead, writer: &mut impl Write) -> io::Result<()> {
    let mut magic_byte = [0u8; 1];
    if reader.read(&mut magic_byte)? == 0 {
        // Empty input is considered valid (results in empty output)
        return Ok(()); 
    }

    if magic_byte[0] != RLE_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid RLE magic byte",
        ));
    }

    let mut buffer = [0u8; 2]; // Read byte and count pair
    let mut output_buffer = Vec::with_capacity(BUFFER_SIZE);

    while reader.read_exact(&mut buffer).is_ok() {
        let byte_val = buffer[0];
        let count = buffer[1] as usize;
        
        if count == 0 {
             return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid RLE sequence: count cannot be zero.",
            ));
        }

        // Write the byte `count` times, buffering the output
        for _ in 0..count {
            output_buffer.push(byte_val);
            if output_buffer.len() == BUFFER_SIZE {
                writer.write_all(&output_buffer)?;
                output_buffer.clear();
            }
        }
    }
    
    // Write any remaining data in the output buffer
    if !output_buffer.is_empty() {
        writer.write_all(&output_buffer)?;
    }

    writer.flush()?;
    Ok(())
}

// --- Buffer-based helper functions for testing ---

#[allow(dead_code)]
/// Compresses a byte slice using RLE (Buffer-based wrapper).
pub fn compress(input: &[u8]) -> io::Result<Vec<u8>> {
    let mut reader = BufReader::new(Cursor::new(input));
    let mut compressed_buf = Vec::new();
    compress_rle(&mut reader, &mut compressed_buf)?;
    Ok(compressed_buf)
}

#[allow(dead_code)]
/// Decompresses a byte slice using RLE (Buffer-based wrapper).
pub fn decompress(input: &[u8]) -> io::Result<Vec<u8>> {
    let mut reader = BufReader::new(Cursor::new(input));
    let mut decompressed_buf = Vec::new();
    decompress_rle(&mut reader, &mut decompressed_buf)?;
    Ok(decompressed_buf)
}


#[cfg(test)]
mod tests {
    use super::*; // Imports functions from the outer scope (including buffer helpers)

    // --- Tests for Buffer-based Helpers (as requested) ---
    
    #[test]
    fn test_rle_buffer_roundtrip_exact() {
        let input = b"AAABBBCCCCCDDDDE";
        let compressed = compress(input).expect("Buffer compression failed");
        
        // Optional: Check expected compressed format if stable
        // let expected_compressed = vec![RLE_MAGIC, 65,3, 66,3, 67,5, 68,4, 69,1];
        // assert_eq!(compressed, expected_compressed);

        let decompressed = decompress(&compressed).expect("Buffer decompression failed");
        assert_eq!(input.to_vec(), decompressed, "Exact RLE buffer roundtrip failed");
    }
}