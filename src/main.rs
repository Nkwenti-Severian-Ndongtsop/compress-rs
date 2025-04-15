use std::{
    fs,
    io::{self, Read, Write},
};

use anyhow::{Context, Result};
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
    /// Compress one or more files
    Compress {
        /// Input file paths (use - for stdin, can specify multiple files)
        #[arg(required = true)]
        inputs: Vec<String>,
        /// Output file path (use - for stdout)
        output: String,
        /// Use RLE compression
        #[arg(long)]
        rle: bool,
        /// Use LZ77 compression
        #[arg(long)]
        lz: bool,
    },
    /// Decompress a file
    Decompress {
        /// Input file path (use - for stdin)
        input: String,
        /// Output file path (use - for stdout)
        output: String,
        /// Use RLE decompression
        #[arg(long)]
        rle: bool,
        /// Use LZ77 decompression
        #[arg(long)]
        lz: bool,
    },
}

fn read_input(input: &str) -> Result<Vec<u8>> {
    if input == "-" {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        Ok(buffer)
    } else {
        fs::read(input).with_context(|| format!("Failed to read input file: {}", input))
    }
}

fn write_output(output: &str, data: &[u8]) -> Result<()> {
    if output == "-" {
        io::stdout()
            .write_all(data)
            .context("Failed to write to stdout")?;
    } else {
        fs::write(output, data).with_context(|| format!("Failed to write output file: {}", output))?;
    }
    Ok(())
}

fn detect_algorithm(data: &[u8]) -> Result<&'static str> {
    if data.is_empty() {
        return Err(anyhow::anyhow!("Empty input data"));
    }

    match data[0] {
        0x52 => Ok("rle"),
        0x4C => Ok("lz"),
        _ => Err(anyhow::anyhow!("Unknown compression algorithm")),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compress {
            inputs,
            output,
            rle,
            lz,
        } => {
            if inputs.len() > 1 && output != "-" {
                return Err(anyhow::anyhow!(
                    "Cannot specify multiple input files with a single output file (except stdout)"
                ));
            }

            let compress_fn = if rle {
                compress_rle
            } else if lz {
                compress_lz
            } else {
                return Err(anyhow::anyhow!("Please specify either --rle or --lz"));
            };

            for input in &inputs {
                let data = read_input(input)?;
                let compressed = compress_fn(&data);
                
                if output == "-" {
                    write_output(&output, &compressed)?;
                } else {
                    // For multiple files, append the compression extension to the output filename
                    let output_path = if inputs.len() > 1 {
                        format!("{}.{}", input, if rle { "rle" } else { "lz" })
                    } else {
                        output.clone()
                    };
                    write_output(&output_path, &compressed)?;
                }
            }
        }
        Commands::Decompress {
            input,
            output,
            rle,
            lz,
        } => {
            let data = read_input(&input)?;

            let decompressed = if rle {
                decompress_rle(&data)?
            } else if lz {
                decompress_lz(&data)?
            } else {
                // Try to detect the algorithm
                match detect_algorithm(&data) {
                    Ok("rle") => decompress_rle(&data)?,
                    Ok("lz") => decompress_lz(&data)?,
                    Ok(_algorithm) => unreachable!(),
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Could not detect compression algorithm. Please specify --rle or --lz: {}",
                            e
                        ));
                    }
                }
            };

            write_output(&output, &decompressed)?;
        }
    }

    Ok(())
} 