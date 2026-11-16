use lxoxide::{LuxologyFile, Chunk};
use::lxoxide::primitives::ID4;
use::binrw::NullString;
use lxoxide::meta::{ApplicationVersion, Version};
use lxoxide::animation::Bake;
use lxoxide::item::{Item, ItemTag};
use std::io::Cursor;
use std::path::PathBuf;
use std::collections::HashMap;

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

#[test]
fn falling_cube() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/falling_cube.lxo");

    let lxo = LuxologyFile::from_path(&path).unwrap();

    // I am guessing this is telling us the version of the fileformat
    assert_eq!(lxo.version(), Some(&Version{
        major: 16, minor: 0, application: NullString::from("nexus 16 by The Foundry")
    }));

    // And this is just information about the application that made it
    assert_eq!(lxo.application_version(), Some(&ApplicationVersion{
        base: 3000, 
        major: 3070, 
        minor: 0, 
        build: 695016, 
        application: NullString::from("Modo 16.1v8")
    }));
    
    // Get all bakes, should be four BAKE chunks in total
    let bakes: Vec<&Bake> = lxo.chunks().iter().filter_map(|c| match c {
        Chunk::Bake(bake) => Some(bake),
        _ => None,
    }).collect();

    // We should have four BAKE chunks
    assert_eq!(bakes.len(), 4);

    // bake.reference, refers to the id of the item, so first we build a hashmap
    // to quicker look up items from their id
    let item_map: HashMap<u32, &Item> = lxo.chunks().iter().filter_map(|c| match c {
        Chunk::Item(item) => Some((item.id, item)),
        _ => None
    }).collect();

    // All bakes should have the same simulation start time and samples per second
    let expected_start_time = 1.0f32 / 24.0f32;
    let expected_samples_per_second = 24.0f32;
    for bake in &bakes {
        assert_eq!(bake.start_time, expected_start_time);
        assert_eq!(bake.samples_per_second, expected_samples_per_second);
    }

    assert_eq!(bakes[0].name, "currentS".into());
    assert_eq!(bakes[0].kind, "time".into());
    let scene = item_map.get(&bakes[0].reference)
        .expect("Expected to find an item from the reference on the BAKE chunk");
    assert_eq!(scene.kind, "scene".into());

    assert!(bakes[0].samples.iter().all(|&s| s == 0u8));

    assert_eq!(bakes[1].name, "compute".into());
    assert_eq!(bakes[1].kind, "integer".into());
    let solver = item_map.get(&bakes[1].reference)
        .expect("Expected to find an item from the reference on the BAKE chunk");
    assert_eq!(solver.kind, "solver".into());

    assert!(bakes[1].samples.iter().all(|&s| s == 0u8));

    // The two last refs from bakes are to Cube mesh
    assert_eq!(bakes[2].reference, bakes[3].reference);
    let cube = item_map.get(&bakes[2].reference)
        .expect("Third bake chunk should reference the Cube mesh item");
    assert_eq!(cube.name, "Cube".into());
    assert_eq!(cube.kind, "mesh".into());

    // As the cubes are simulated falling, we should see both wpos as well as wrot
    // data change over time
    assert!(!bakes[2].samples.iter().all(|&s| s == 0u8));
    assert!(!bakes[3].samples.iter().all(|&s| s == 0u8));

    // Cube has one item tag, marking it as a rigid body
    let mut itag_iter = cube.tags();
    assert_eq!(
        itag_iter.next(),
        Some(&ItemTag{
            kind:ID4::from_bytes(*b"DTYP").unwrap(),
            tag:NullString::from("rigid")}
            )
        );
    assert_eq!(itag_iter.next(), None);

    // TODO: find ground plane, should be item id mesh027, and do asserts on it

    let mut writer = Cursor::new(vec![]);
    lxo.to_writer(&mut writer).unwrap();
    let bytes = std::fs::read(&path).unwrap();

    assert_eq!(writer.into_inner(), bytes);
}
