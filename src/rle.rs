use std::io::{self, Read, Write, BufReader, BufRead, ErrorKind};

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_rle_streaming_roundtrip() -> io::Result<()> {
        let input_data = b"AAABBBCCCCCDDDDEFFFGAAAAAAAAA".to_vec();
        let mut compressed_buf = Vec::new();
        let mut reader = BufReader::new(Cursor::new(input_data.clone()));

        compress_rle(&mut reader, &mut compressed_buf)?;

        // Basic check on compressed data
        assert!(compressed_buf.len() > 1); 
        assert_eq!(compressed_buf[0], RLE_MAGIC);

        let mut decompressed_buf = Vec::new();
        let mut compressed_reader = BufReader::new(Cursor::new(compressed_buf));
        decompress_rle(&mut compressed_reader, &mut decompressed_buf)?;

        assert_eq!(input_data, decompressed_buf);
        Ok(())
    }

    #[test]
    fn test_decompress_invalid_magic() {
        let invalid_data = vec![0x00, 0x41, 0x03]; // Incorrect magic byte
        let mut reader = BufReader::new(Cursor::new(invalid_data));
        let mut writer = Vec::new();
        let result = decompress_rle(&mut reader, &mut writer);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().kind(), ErrorKind::InvalidData);
    }

     #[test]
    fn test_decompress_incomplete_pair() {
        let invalid_data = vec![RLE_MAGIC, 0x41]; // Missing count
        let mut reader = BufReader::new(Cursor::new(invalid_data));
        let mut writer = Vec::new();
        // Expect read_exact to fail within decompress_rle
        let result = decompress_rle(&mut reader, &mut writer);
         assert!(result.is_err());
         // The error might be UnexpectedEof depending on BufRead implementation, 
         // but the cause is invalid data structure.
    }

    #[test]
    fn test_compress_empty() -> io::Result<()>{
        let input_data = b"".to_vec();
        let mut compressed_buf = Vec::new();
        let mut reader = BufReader::new(Cursor::new(input_data.clone()));
        compress_rle(&mut reader, &mut compressed_buf)?;
        assert_eq!(compressed_buf, vec![RLE_MAGIC]);

        let mut decompressed_buf = Vec::new();
        let mut compressed_reader = BufReader::new(Cursor::new(compressed_buf));
        decompress_rle(&mut compressed_reader, &mut decompressed_buf)?;
        assert_eq!(input_data, decompressed_buf);
        Ok(())
    }
} 