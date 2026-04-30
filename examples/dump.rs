use lxoxide::{LuxologyFile, Chunk};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path>", args[0]);
        std::process::exit(1);
    }
    let path = std::path::PathBuf::from(&args[1]);

    let file = LuxologyFile::from_path(&path)?;

    let header = &file.header;
    println!("{:?} file content size: {} bytes", header.kind, header.size);

    for chunk in &file.chunks {
        match chunk {
            Chunk::VRSN(version) => println!("{}", version),
            Chunk::APPV(application_version) => println!("{}", application_version),
            Chunk::ENCO(encoding) => println!("Encoding: {}", encoding),
            Chunk::LAYR(layer) => println!("Layer {}", layer),
            Chunk::PNTS(points) => println!("Points: {}", points.0.len()),
            Chunk::POLS(polygon_list) => {
                println!("{} {} type Polygons", polygon_list.polygons.len(), polygon_list.kind)
            },
            Chunk::ITEM(item) => println!("Item {}", item.name.0), 
            Chunk::Unknown{kind: k, position: p, size: s} => {
                println!("{} position: {}, chunk size: {}", k, p, s);
            },
        }
    }

    Ok(())
}
