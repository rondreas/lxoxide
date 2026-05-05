use lxoxide::{Chunk, LuxologyFile};
use std::iter::zip;

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
            Chunk::TAGS(tags) => {
                println!("Item Tags: ");
                for tag in &tags.tags {
                    println!("  {tag}");
                }
            }
            Chunk::CHNM(channel_names) => {
                println!("{} Channel names:", channel_names.count);
                for (_, name) in zip(0..5, &channel_names.names) {
                    println!("  {name}");
                }
                println!("  ...");
            }
            Chunk::LAYR(layer) => println!("Layer {}", layer),
            Chunk::PNTS(points) => println!("Points: {}", points.0.len()),
            Chunk::VMPA(params) => {
                println!("Vertex Map Parameters:");
                println!("  UV Subdiv type: {:?}", params.uv_subdivision);
                println!("  Sketch Color: {}", params.sketch_color);
            }
            Chunk::VMAP(vmap) => {
                println!("{} Vertex Map {}", vmap.kind, vmap.name)
            }
            Chunk::POLS(polygon_list) => {
                println!(
                    "{} {} type Polygons",
                    polygon_list.polygons.len(),
                    polygon_list.kind
                )
            }
            Chunk::VMAD(vmad) => {
                println!("{} Discont. Vertex Map {}", vmad.kind, vmad.name)
            }
            Chunk::PTAG(ptag) => {
                println!("{} Polygon Tag", ptag.kind)
            }
            Chunk::ITEM(item) => println!("Item {}", item.name.to_string()),
            Chunk::ENVL(envelope) => println!("{} Envelope", envelope.kind),
            Chunk::ACTN(action) => println!("{} action", action.name),
            Chunk::AANI(audio) => {
                println!("Audio");
                if audio.settings.is_some() {
                    let settings = audio.settings.as_ref().unwrap();
                    println!(
                        "  Settings loop: {}, mute: {}, scrub: {}, start: {}",
                        settings.r#loop, settings.mute, settings.scrub, settings.start
                    );
                }
            }
            Chunk::Unknown {
                kind: k,
                position: p,
                size: s,
            } => {
                println!("{} position: {}, chunk size: {}", k, p, s);
            }
        }
    }

    Ok(())
}
