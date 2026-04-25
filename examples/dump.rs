use lxoxide::LuxologyFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from("/home/andreas/Documents/lxo/cube.lxo");

    let file = LuxologyFile::from_path(&path)?;

    let header = &file.header;
    println!("FORM size: {} bytes, type: {:?}", header.size, header.kind);

    let mut offset: u64 = 0;
    for chunk in &file.chunks {
        println!("{} size: {}", chunk.kind, chunk.size);
    }

    Ok(())
}
