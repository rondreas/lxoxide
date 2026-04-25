use lxoxide::LuxologyFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from("/home/andreas/Documents/lxo/cube.lxo");

    let file = LuxologyFile::from_path(&path)?;

    let header = &file.header;
    println!("{:?} file content size: {} bytes", header.kind, header.size);

    let mut offset: u64 = 12;  // after reading iff header, we have read 12 bytes
    for chunk in &file.chunks {
        println!("{} chunk at position: {}, size: {}", chunk.kind, offset, chunk.size + 8);
        offset += (chunk.size as u64 + 8);
    }

    Ok(())
}
