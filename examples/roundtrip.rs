use lxoxide::LuxologyFile;
use std::io::Cursor;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path> [output path]", args[0]);
        std::process::exit(1);
    }
    let path = PathBuf::from(&args[1]);

    let lxo = match LuxologyFile::from_path(&path) {
        Ok(lxo) => lxo,
        Err(e) => {
            eprintln!("Could not parse {}: {e}", path.display());
            std::process::exit(1);
        }
    };

    let mut writer = Cursor::new(vec![]);
    if let Err(e) = lxo.to_writer(&mut writer) {
        eprintln!("Could not serialize {}: {e}", path.display());
        std::process::exit(1);
    }
    let roundtripped = writer.into_inner();

    if let Some(output) = args.get(2) {
        if let Err(e) = lxo.to_path(output) {
            eprintln!("Could not write {output}: {e}");
            std::process::exit(1);
        }
        println!("Wrote re-serialized file to {output}");
    }

    let original = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Could not read {}: {e}", path.display());
            std::process::exit(1);
        }
    };

    if roundtripped == original {
        println!(
            "Roundtrip OK: {} bytes identical to {}",
            original.len(),
            path.display()
        );
    } else {
        eprintln!(
            "Roundtrip MISMATCH: wrote {} bytes, original {} has {} bytes",
            roundtripped.len(),
            path.display(),
            original.len()
        );
        std::process::exit(1);
    }
}
