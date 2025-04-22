use std::collections::VecDeque;
use std::io::{self, BufRead, Write};

const WINDOW_SIZE: usize = 4096;
const LOOKAHEAD_BUFFER_SIZE: usize = 18;
const MIN_MATCH_LENGTH: usize = 3;
const MAX_MATCH_LENGTH: usize = LOOKAHEAD_BUFFER_SIZE;

pub fn compress_lz(input: &mut dyn BufRead, output: &mut dyn Write) -> io::Result<()> {
    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer)?;

    let mut pos = 0;
    while pos < buffer.len() {
        let window_start = if pos >= WINDOW_SIZE { pos - WINDOW_SIZE } else { 0 };
        let search_buffer = &buffer[window_start..pos];
        let lookahead_buffer = &buffer[pos..(pos + LOOKAHEAD_BUFFER_SIZE).min(buffer.len())];

        let mut best_match_offset = 0;
        let mut best_match_length = 0;

        for i in 0..search_buffer.len() {
            let mut current_length = 0;
            while current_length < lookahead_buffer.len()
                && current_length < MAX_MATCH_LENGTH
                && i + current_length < search_buffer.len()
                && search_buffer[i + current_length] == lookahead_buffer[current_length]
            {
                current_length += 1;
            }

            if current_length >= MIN_MATCH_LENGTH && current_length > best_match_length {
                best_match_length = current_length;
                best_match_offset = search_buffer.len() - i;
            }
        }

        let (next_byte_flag, next_byte) = if best_match_length < lookahead_buffer.len() {
            (1u8, lookahead_buffer[best_match_length])
        } else {
            (0u8, 0) // next_byte will not be written if flag is 0
        };

        // Write token
        output.write_all(&(best_match_offset as u16).to_le_bytes())?;
        output.write_all(&(best_match_length as u8).to_le_bytes())?;
        output.write_all(&[next_byte_flag])?;
        if next_byte_flag == 1 {
            output.write_all(&[next_byte])?;
        }

        pos += best_match_length + if next_byte_flag == 1 { 1 } else { 0 };
    }

    Ok(())
}

pub fn decompress_lz(input: &mut dyn BufRead, output: &mut dyn Write) -> io::Result<()> {
    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer)?;

    let mut pos = 0;
    let mut history_buffer = VecDeque::with_capacity(WINDOW_SIZE);

    while pos + 4 <= buffer.len() {
        let offset = u16::from_le_bytes([buffer[pos], buffer[pos + 1]]) as usize;
        let length = buffer[pos + 2] as usize;
        let flag = buffer[pos + 3];
        pos += 4;

        let next_byte = if flag == 1 {
            if pos < buffer.len() {
                let b = buffer[pos];
                pos += 1;
                Some(b)
            } else {
                None
            }
        } else {
            None
        };

        let start_index = history_buffer.len().saturating_sub(offset);

        for i in 0..length {
            if offset == 0 || start_index + (i % offset) >= history_buffer.len() {
                break;
            }
            let byte_to_copy = history_buffer[start_index + (i % offset)];
            output.write_all(&[byte_to_copy])?;
            history_buffer.push_back(byte_to_copy);
            if history_buffer.len() > WINDOW_SIZE {
                history_buffer.pop_front();
            }
        }

        if let Some(b) = next_byte {
            output.write_all(&[b])?;
            history_buffer.push_back(b);
            if history_buffer.len() > WINDOW_SIZE {
                history_buffer.pop_front();
            }
        }
    }

    Ok(())
}
