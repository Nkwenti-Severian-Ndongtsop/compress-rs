use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
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
    /// Compress a file
    Compress {
        /// Input file path (use - for stdin)
        input: String,
        /// Output file path (use - for stdout)
        output: String,
        /// Use RLE compression
        #[arg(long)]
        rle: bool,
        /// Use LZ77 compression
        #[arg(long)]
        lz: bool,
    },
    /// Compress a folder
    CompressFolder {
        /// Input folder path
        input: String,
        /// Output file path
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
    /// Decompress a folder
    DecompressFolder {
        /// Input file path
        input: String,
        /// Output folder path
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

fn collect_files(path: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut files = Vec::new();
    
    if path.is_file() {
        let content = fs::read(path)?;
        files.push((path.to_path_buf(), content));
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let content = fs::read(&path)?;
                files.push((path, content));
            }
        }
    }
    
    Ok(files)
}

fn compress_folder(files: Vec<(PathBuf, Vec<u8>)>, algorithm: &str) -> Result<Vec<u8>> {
    let mut archive = Vec::new();
    
    // Add archive header with magic byte
    let magic_byte = match algorithm {
        "rle" => 0x52,
        "lz" => 0x4C,
        _ => return Err(anyhow::anyhow!("Invalid algorithm")),
    };
    archive.push(magic_byte);
    
    // Add number of files
    archive.push(files.len() as u8);
    
    for (path, content) in files {
        // Add file header: [path_len: u8][path_bytes][content_len: u32][content_bytes]
        let path_str = path.to_string_lossy().into_owned();
        let path_bytes = path_str.as_bytes();
        archive.push(path_bytes.len() as u8);
        archive.extend_from_slice(path_bytes);
        
        let content_len = content.len() as u32;
        archive.extend_from_slice(&content_len.to_le_bytes());
        
        let compressed = match algorithm {
            "rle" => compress_rle(&content),
            "lz" => compress_lz(&content),
            _ => return Err(anyhow::anyhow!("Invalid algorithm")),
        };
        
        archive.extend_from_slice(&compressed);
    }
    
    Ok(archive)
}

fn decompress_folder(data: &[u8], output_dir: &Path, algorithm: &str) -> Result<()> {
    if data.is_empty() {
        return Err(anyhow::anyhow!("Empty archive"));
    }
    
    // Skip the magic byte since it's already checked by detect_algorithm
    let mut pos = 1;
    let num_files = data[pos] as usize;
    pos += 1;
    
    for _ in 0..num_files {
        // Read file header
        let path_len = data[pos] as usize;
        pos += 1;
        
        let path_bytes = &data[pos..pos + path_len];
        pos += path_len;
        
        let path_str = String::from_utf8_lossy(path_bytes).into_owned();
        let output_path = output_dir.join(path_str);
        
        // Create parent directories if they don't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Read content length
        let content_len = u32::from_le_bytes(data[pos..pos + 4].try_into()?) as usize;
        pos += 4;
        
        // Read and decompress content
        let compressed = &data[pos..pos + content_len];
        pos += content_len;
        
        let decompressed = match algorithm {
            "rle" => decompress_rle(compressed)?,
            "lz" => decompress_lz(compressed)?,
            _ => return Err(anyhow::anyhow!("Invalid algorithm")),
        };
        
        fs::write(output_path, decompressed)?;
    }
    
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compress {
            input,
            output,
            rle,
            lz,
        } => {
            let data = read_input(&input)?;

            let compressed = if rle {
                compress_rle(&data)
            } else if lz {
                compress_lz(&data)
            } else {
                return Err(anyhow::anyhow!("Please specify either --rle or --lz"));
            };

            write_output(&output, &compressed)?;
        }
        Commands::CompressFolder {
            input,
            output,
            rle,
            lz,
        } => {
            let files = collect_files(Path::new(&input))?;
            
            let algorithm = if rle {
                "rle"
            } else if lz {
                "lz"
            } else {
                return Err(anyhow::anyhow!("Please specify either --rle or --lz"));
            };
            
            let compressed = compress_folder(files, algorithm)?;
            write_output(&output, &compressed)?;
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
        Commands::DecompressFolder {
            input,
            output,
            rle,
            lz,
        } => {
            let data = read_input(&input)?;
            let output_dir = Path::new(&output);
            
            // Create output directory if it doesn't exist
            fs::create_dir_all(output_dir)?;
            
            let algorithm = if rle {
                "rle"   
            } else if lz {
                "lz"
            } else {
                // Try to detect the algorithm
                match detect_algorithm(&data) {
                    Ok("rle") => "rle",
                    Ok("lz") => "lz",
                    Ok(_) => unreachable!(),
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Could not detect compression algorithm. Please specify --rle or --lz: {}",
                            e
                        ));
                    }
                }
            };
            
            decompress_folder(&data, output_dir, algorithm)?;
        }
    }

    Ok(())
} 