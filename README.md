# lxoxide

Rust library for parsing Luxology file formats used by Foundry's Modo 3D software.

## Supported Formats

| Extension | Type | Description |
|-----------|------|-------------|
| `.lxo` | `LXOB` | Scene file |
| `.lxp` | `LXPR` | Preset assembly |
| `.lxe` | `LXPE` | Preset environment |
| `.lxl` | `LXPM` | Preset item |

## Usage

```toml
[dependencies]
lxoxide = "0.1.0"
```

```rust
use lxoxide::LuxologyFile;

let file = LuxologyFile::from_path("path/to/scene.lxo")?;

// Access parsed data
for layer in &file.layers {
    // Points, polygons, vertex maps, etc.
}

for trisurf in &file.trisurfs {
    // Triangulated surface data
}

for item in &file.items {
    // Scene items with animated channels
}
```

## Parsed Data

The `LuxologyFile` struct provides access to:

- **Geometry** – Layers with points (`PNTS`), polygons (`POLS`), bounding boxes (`BBOX`), vertex maps (`VMAP`), and vertex map parameters (`VMPA`)
- **TriSurfaces** – Triangulated mesh data with vertices, triangles, vectors, and tags
- **Items** – Scene items with transform channels, visibility, and custom properties
- **Animation** – Envelopes (`ENVL`) and actions (`ACTN`)
- **Metadata** – File version, application version, description, encoding, tags, and channel names
- **Embedded Data** – Binary data blocks (`DATA`) and audio (`AANI`)

## Examples

```bash
# Dump chunk structure of an LXO file
cargo run --example dump path/to/file.lxo
```

## Format Details

The LXO format is an IFF-based chunk format derived from Lightwave's LWO2. All multi-byte values are big-endian, and data is padded to even byte boundaries.

See [`docs/lxo_format_spec.md`](docs/lxo_format_spec.md) for the official Luxology format specification (v4.2).

## Dependencies

- [binrw](https://crates.io/crates/binrw) – Binary read/write with DSL macros
- [thiserror](https://crates.io/crates/thiserror) – Error derive macros
- [bitflags](https://crates.io/crates/bitflags) – Type-safe bitflags
