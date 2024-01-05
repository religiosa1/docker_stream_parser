mod args;
mod docker_decoder_chunk_writer;
mod docker_decoder_error;
mod docker_stream_decoder;
mod frame_header;

use std::error::Error;
use std::{
    fs::File,
    io::{BufReader, Read},
};

use args::Args;
use docker_decoder_error::DockerDecoderError;
use frame_header::StreamType;

const BUFFER_SIZE: usize = 8192;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let mut buffer = [0u8; BUFFER_SIZE];

    for filename in &args.files {
        let mut decoder = docker_stream_decoder::DockerStreamDecoder::new();
        let file: Box<dyn Read> = match filename.as_str() {
            "-" => Box::new(std::io::stdin()),
            _ => Box::new(File::open(filename)?),
        };
        let mut reader = BufReader::new(file);

        let mut chunk_writer = docker_decoder_chunk_writer::DockerDecoderChunkWriter::new(&args)?;

        let bytes_read = match reader.read(&mut buffer) {
            Ok(n) => n,
            Err(err) => {
                if !args.silent {
                    eprintln!("Error reading from file {}: {}", filename, err);
                }
                return Err(Box::new(err));
            }
        };

        for chunk_result in decoder.decode(&buffer[0..bytes_read]) {
            match chunk_result {
                Ok(chunk) => {
                    if let Err(_) = StreamType::try_from(chunk.stream_type) {
                        if !args.silent {
                            eprintln!("Incorrect docker stream type {}", chunk.stream_type);
                        }
                        if args.fatal {
                            return Err(Box::new(DockerDecoderError::IncorrectFrameType(
                                chunk.stream_type,
                            )));
                        }
                    }
                    chunk_writer.write(chunk)?;
                }
                Err(err) => {
                    if !args.silent {
                        eprintln!("Error processing docker stream {}", err);
                    }
                    if args.fatal {
                        return Err(Box::new(err));
                    }
                }
            }
        }
    }
    Ok(())
}