use lxoxide::LuxologyFile;
use std::io::Cursor;
use std::path::PathBuf;

#[test]
fn default() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/default.lxo");

    let lxo = LuxologyFile::from_path(&path).unwrap();

    let mut writer = Cursor::new(vec![]);
    lxo.to_writer(&mut writer).unwrap();
    let bytes = std::fs::read(&path).unwrap();

    assert_eq!(writer.into_inner(), bytes);
}

#[test]
fn cube() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/cube.lxl");

    let lxo = LuxologyFile::from_path(&path).unwrap();

    let mut writer = Cursor::new(vec![]);
    lxo.to_writer(&mut writer).unwrap();
    let bytes = std::fs::read(&path).unwrap();

    assert_eq!(writer.into_inner(), bytes);
}
