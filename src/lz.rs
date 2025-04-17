use std::io::{self, Read, Write, BufRead, ErrorKind};
use std::collections::VecDeque;

const LZ_MAGIC: u8 = 0x4C;
const MAX_SEARCH_BUFFER_SIZE: usize = 4096; // Size of the look-behind buffer (adjust as needed)
const MAX_LOOKAHEAD_BUFFER_SIZE: usize = 18;  // Max match length (limited by LZ77 format)
const MIN_MATCH_LENGTH: usize = 3; // Minimum length to encode as (offset, length)

/// Compresses data from a reader to a writer using LZ77 algorithm
/// 
/// # Arguments
/// * `reader` - A mutable reference to a type implementing BufRead
/// * `writer` - A mutable reference to a type implementing Write
/// 
/// # Returns
/// An I/O Result indicating success or failure
pub fn compress_lz(reader: &mut impl BufRead, writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(&[LZ_MAGIC])?;

    let mut search_buffer: VecDeque<u8> = VecDeque::with_capacity(MAX_SEARCH_BUFFER_SIZE);
    let mut lookahead_buffer = Vec::new();
    let mut bytes_read = 0;

    // Fill initial lookahead buffer
    let initial_read = reader.take(MAX_LOOKAHEAD_BUFFER_SIZE as u64).read_to_end(&mut lookahead_buffer)?;
    bytes_read += initial_read;

    while !lookahead_buffer.is_empty() {
        let mut best_match_offset: usize = 0;
        let mut best_match_length: usize = 0;

        // Search for the longest match in the search buffer
        let search_limit = search_buffer.len();
        let lookahead_limit = lookahead_buffer.len();

        for start_offset in 0..search_limit {
            let mut current_length = 0;
            while current_length < lookahead_limit && 
                  start_offset + current_length < search_limit && // Boundary check for search buffer access
                  search_buffer[start_offset + current_length] == lookahead_buffer[current_length] {
                current_length += 1;
            }
            
            // Use the most recent match if lengths are equal
            if current_length >= best_match_length {
                best_match_length = current_length;
                best_match_offset = search_limit - start_offset; // Calculate offset relative to current position
            }
        }

        // If a good enough match is found, write (offset, length) token
        if best_match_length >= MIN_MATCH_LENGTH && best_match_offset <= 255 && best_match_length <= 255 {
             // Write offset and length (ensure they fit in u8)
            writer.write_all(&[best_match_offset as u8, best_match_length as u8])?;

            // Shift buffers
            for _ in 0..best_match_length {
                if let Some(byte) = lookahead_buffer.pop_front() {
                    search_buffer.push_back(byte);
                    if search_buffer.len() > MAX_SEARCH_BUFFER_SIZE {
                        search_buffer.pop_front();
                    }
                }
                 // Read next byte into lookahead if available
                let mut next_byte_buf = [0u8; 1];
                if reader.read(&mut next_byte_buf)? > 0 {
                    lookahead_buffer.push_back(next_byte_buf[0]);
                    bytes_read += 1;
                }
            }

        } else {
            // No good match found, write literal (0, 0, byte)
            let literal_byte = lookahead_buffer.pop_front().unwrap(); // Safe because loop condition ensures not empty
            writer.write_all(&[0, 0, literal_byte])?;

            // Add literal to search buffer
            search_buffer.push_back(literal_byte);
             if search_buffer.len() > MAX_SEARCH_BUFFER_SIZE {
                search_buffer.pop_front();
            }

             // Read next byte into lookahead if available
            let mut next_byte_buf = [0u8; 1];
             if reader.read(&mut next_byte_buf)? > 0 {
                lookahead_buffer.push_back(next_byte_buf[0]);
                bytes_read += 1;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

/// Decompresses LZ77-encoded data from a reader to a writer
pub fn decompress_lz(reader: &mut impl BufRead, writer: &mut impl Write) -> io::Result<()> {
    let mut magic_byte = [0u8; 1];
    if reader.read(&mut magic_byte)? == 0 {
        return Ok(()); // Empty input
    }
    if magic_byte[0] != LZ_MAGIC {
        return Err(io::Error::new(ErrorKind::InvalidData, "Invalid LZ77 magic byte"));
    }

    let mut output_buffer: VecDeque<u8> = VecDeque::with_capacity(MAX_SEARCH_BUFFER_SIZE + MAX_LOOKAHEAD_BUFFER_SIZE);
    let mut token_buffer = [0u8; 2]; // For offset and length

    loop {
        match reader.read_exact(&mut token_buffer) {
            Ok(()) => {
                let offset = token_buffer[0] as usize;
                let length = token_buffer[1] as usize;

                if offset == 0 && length == 0 {
                    // Literal byte follows
                    let mut literal_byte = [0u8; 1];
                    if reader.read_exact(&mut literal_byte).is_err() {
                        return Err(io::Error::new(ErrorKind::UnexpectedEof, "Unexpected EOF after literal token"));
                    }
                    let byte_val = literal_byte[0];
                    writer.write_all(&[byte_val])?;
                    output_buffer.push_back(byte_val);
                    if output_buffer.len() > MAX_SEARCH_BUFFER_SIZE { // Maintain history buffer size
                       output_buffer.pop_front();
                    }
                } else {
                    // (offset, length) pair
                    if offset == 0 || length == 0 {
                         return Err(io::Error::new(ErrorKind::InvalidData, format!("Invalid LZ77 token (offset={}, length={})", offset, length)));
                    }
                    if offset > output_buffer.len() {
                        return Err(io::Error::new(ErrorKind::InvalidData, format!("Invalid LZ77 offset {} > history size {}", offset, output_buffer.len())));
                    }

                    let start_index = output_buffer.len() - offset;
                    let mut bytes_to_write = Vec::with_capacity(length);
                    for i in 0..length {
                         // Handle potential wrap-around within the VecDeque if offset is large
                        let index = (start_index + i) % output_buffer.capacity(); // Modulo capacity might be wrong if len != cap
                        // Safer access relative to start:
                        let byte_to_copy = output_buffer[start_index + i];
                        bytes_to_write.push(byte_to_copy);
                    }
                    
                    writer.write_all(&bytes_to_write)?;
                    for byte_val in bytes_to_write {
                         output_buffer.push_back(byte_val);
                        if output_buffer.len() > MAX_SEARCH_BUFFER_SIZE {
                           output_buffer.pop_front();
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => {
                // Expected EOF if no more tokens are left
                break;
            }
            Err(e) => {
                // Other read errors
                return Err(e);
            }
        }
    }

    writer.flush()?;
    Ok(())
}

// TODO: Add streaming tests for LZ77 similar to RLE tests 