use lxoxide::LuxologyFile;
use std::path::PathBuf;

#[test]
fn cube() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/cube.lxo");

    let lxo = LuxologyFile::from_path(path).unwrap();

    assert_eq!(lxo.layers.len(), 1);

    let layer = &lxo.layers[0];
    let number_of_vertices: usize = layer
        .geometry
        .points
        .as_ref()
        .and_then(|points| Some(points.len()))
        .expect("The layer should have points in geometry");
    assert_eq!(number_of_vertices, 8);

    // We can get the item for the layer, using reference as index into lxo.items
    let cube_item = &lxo.items[layer.reference as usize];
    assert_eq!(cube_item.kind, "mesh".into());
    assert_eq!(cube_item.visible_name, Some("Mesh".into()));

    // An lxo should only have one item of kind scene
    let number_of_scenes = lxo
        .items
        .iter()
        .filter(|item| item.kind == "scene".into())
        .count();
    assert_eq!(number_of_scenes, 1);
}
