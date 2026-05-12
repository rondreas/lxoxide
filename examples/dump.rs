use lxoxide::{Header, ChunkHeader};
use std::fs::File;
use std::io::{BufReader, Seek};
use binrw::{BinReaderExt, BinRead};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path>", args[0]);
        std::process::exit(1);
    }
    let path = std::path::PathBuf::from(&args[1]);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let header: Header = reader.read_be()?;
    println!("{:?} file content size: {} bytes", header.kind, header.size);

    loop {
        let position = reader.stream_position()?;
        let chunk_header = match ChunkHeader::read_be(&mut reader) {
            Ok(h) => h,
            Err(e) => {
                if e.is_eof() {
                    break;
                }
                return Err(e.into());
            }
        };

        println!("{} position: {}, size: {}", chunk_header.kind, position, chunk_header.size + 8);
        reader.seek_relative(chunk_header.size as i64)?;
    }

    Ok(())
}
