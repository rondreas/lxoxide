use lxoxide::LuxologyFile;
use std::io::Cursor;
use std::path::PathBuf;

#[test]
fn cube() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/cube.lxo");

    let lxo = LuxologyFile::from_path(&path).unwrap();

    let mut writer = Cursor::new(vec![]);
    lxo.to_writer(&mut writer).unwrap();

    // not optimal, but for now saving file contents as bytes for comparision
    let bytes = std::fs::read(&path).unwrap();

    assert_eq!(writer.into_inner(), bytes);
}
