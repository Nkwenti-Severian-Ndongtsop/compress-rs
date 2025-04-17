use std::{
    fs::File,
    io::{self, BufReader, BufWriter, BufRead, Write}
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

mod lz;
mod rle;

use lz::{compress_lz, decompress_lz};
use rle::{compress_rle, decompress_rle};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compress an input file to an output file
    Compress {
        /// Input file path (use - for stdin)
        input: String,
        /// Output file path (use - for stdout)
        output: String,
        /// Use RLE compression (default if not specified)
        #[arg(long, group = "compress_algo")]
        rle: bool,
        /// Use LZ77 compression
        #[arg(long, group = "compress_algo")]
        lz: bool,
    },
    /// Decompress an input file to an output file
    Decompress {
        /// Input file path (use - for stdin)
        input: String,
        /// Output file path (use - for stdout)
        output: String,
         /// Hint to use RLE decompression
        #[arg(long, group = "decompress_algo")]
        rle: bool,
        /// Hint to use LZ77 decompression
        #[arg(long, group = "decompress_algo")]
        lz: bool,
    },
}

// Helper to open input (file or stdin)
fn open_input(path: &str) -> Result<Box<dyn BufRead>> {
    if path == "-" {
        Ok(Box::new(BufReader::new(io::stdin()))) 
    } else {
        let file = File::open(path).with_context(|| format!("Failed to open input file: {}", path))?;
        Ok(Box::new(BufReader::new(file)))
    }
}

// Helper to open output (file or stdout)
fn open_output(path: &str) -> Result<Box<dyn Write>> {
    if path == "-" {
        Ok(Box::new(BufWriter::new(io::stdout())))
    } else {
        let file = File::create(path).with_context(|| format!("Failed to create output file: {}", path))?;
        Ok(Box::new(BufWriter::new(file)))
    }
}

// Function to detect algorithm by peeking at the first byte
fn detect_algorithm_from_stream(reader: &mut dyn BufRead) -> Result<&'static str> {
    let first_byte = reader.fill_buf()?.get(0).copied(); // Peek at first byte
    match first_byte {
        Some(0x52) => Ok("rle"), // RLE_MAGIC
        Some(0x4C) => Ok("lz"),  // LZ_MAGIC
        Some(_) => bail!("Unknown compression algorithm magic byte"),
        None => bail!("Cannot detect algorithm from empty input"),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compress {
            input,
            output,
            mut rle, // Make mutable as we set default
            lz,
        } => {
            // Default to RLE if no algorithm is specified
            if !rle && !lz {
                println!("No compression algorithm specified, defaulting to RLE.");
                rle = true;
            }

            let mut reader = open_input(&input)?;
            let mut writer = open_output(&output)?;

            if rle {
                println!("Compressing {} to {} using RLE...", input, output);
                compress_rle(&mut reader, &mut writer)
                    .with_context(|| format!("RLE compression failed from {} to {}", input, output))?;
            } else {
                // lz must be true
                 println!("Compressing {} to {} using LZ77...", input, output);
                 compress_lz(&mut reader, &mut writer)
                     .with_context(|| format!("LZ77 compression failed from {} to {}", input, output))?;
            }
             println!("Compression successful.");
        }
        Commands::Decompress {
            input,
            output,
            rle,
            lz,
        } => {
            let mut reader = open_input(&input)?;
            let mut writer = open_output(&output)?;

            if rle {
                println!("Decompressing {} to {} using RLE...", input, output);
                decompress_rle(&mut reader, &mut writer)
                   .with_context(|| format!("RLE decompression failed from {} to {}", input, output))?;
            } else if lz {
                 println!("Decompressing {} to {} using LZ77...", input, output);
                decompress_lz(&mut reader, &mut writer)
                     .with_context(|| format!("LZ77 decompression failed from {} to {}", input, output))?;
            } else {
                // No hint provided, detect algorithm
                println!("No algorithm specified, attempting detection...");
                let algorithm = detect_algorithm_from_stream(&mut reader)
                    .with_context(|| format!("Failed to detect algorithm for input {}", input))?;
                
                match algorithm {
                    "rle" => {
                        println!("Detected RLE. Decompressing {} to {}...", input, output);
                         decompress_rle(&mut reader, &mut writer).with_context(|| {
                            format!("RLE decompression failed from {} to {}", input, output)
                        })?;
                    }
                    "lz" => {
                         println!("Detected LZ77. Decompressing {} to {}...", input, output);
                         decompress_lz(&mut reader, &mut writer).with_context(|| {
                             format!("LZ77 decompression failed from {} to {}", input, output)
                        })?;
                    }
                    _ => unreachable!(), // Should be caught by detect_algorithm_from_stream
                }
            }
            println!("Decompression successful.");
        }
    }

    Ok(())
} 