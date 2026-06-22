//! Trisurf geometry parsing.
//!
//! Trisurfs are a simplified geometry representation used in Modo for "Static Mesh" items.
//! Static meshes are non-editable, frozen objects used to streamline workflows with
//! extremely dense meshes by removing the overhead associated with mesh editing,
//! thereby increasing overall performance and minimizing file size.
//!
//! They are created by converting regular mesh items, a process that freezes the mesh
//! and triangulates all geometry, including any subdivided surfaces. Once converted,
//! component-level mesh editing is disabled, and morph maps and deformers are removed.
//!
//! They render like regular meshes and can be positioned by regular item level transformations.
//! However, they cannot be used as dynamic objects (as they produce no collision shape)
//! or as point sources for Replicators.
//!
//! This module provides the structures for parsing the corresponding IFF chunks:
//! `3GRP`, `3SRF`, `VRTS`, `TRIS`, `VVEC`, and `TTGS`.

use crate::ParseError;
use crate::primitives::{ID4, Point};
use crate::utils::read_aligned_nullstring;
use binrw::{BinRead, BinResult, BinWrite, Endian, NullString};
use std::io::{Read, Seek};

/// Trisurf Group Header (`3GRP`).
///
/// The `3GRP` chunk precedes any other trisurf chunks and defines a group of
/// `TriSurfDataHeader` (`3SRF`) chunks. Multiple groups may exist in a file.
#[derive(Debug, BinRead, BinWrite)]
pub struct TriSurfGroupHeader {
    /// Number of trisurfs in the group. Should match the number of `3SRF` chunks following this header.
    pub trisurf_count: u32,
    /// Item reference index that this group is associated with.
    pub item_reference: u32,
    /// Flags for future expansion.
    pub flags: u32,

    #[br(ignore)]
    pub trisurfaces: Vec<TriSurfDataHeader>,
}

/// Trisurf Data Header (`3SRF`).
///
/// The `3SRF` chunk identifies a collection of geometry within a trisurf group.
/// It is followed by its associated vertex positions, triangle indices,
/// vertex vectors, and tags.
#[derive(Debug, BinRead, BinWrite)]
pub struct TriSurfDataHeader {
    /// Number of vertices in the associated `TriSurfVertices` (`VRTS`) chunk.
    pub vertex_count: u32,
    /// Number of triangles in the associated `TriSurfTriangles` (`TRIS`) chunk.
    pub triangle_count: u32,
    /// Number of associated `TriSurfVertexVectors` (`VVEC`) chunks.
    pub vertex_vector_count: u32,
    /// Number of tags in the associated `TriSurfTags` (`TTGS`) chunk.
    pub tag_count: u32,
    /// Flags for future expansion.
    pub flags: u32,

    #[br(ignore)]
    pub vertices: Option<TriSurfVertices>,

    #[br(ignore)]
    pub triangles: Option<TriSurfTriangles>,

    #[br(ignore)]
    pub vectors: Vec<TriSurfVertexVectors>,

    #[br(ignore)]
    pub tags: Option<TriSurfTags>,
}

/// Vertex Position Array (`VRTS`).
///
/// Contains an array of vertex positions for the preceding `TriSurfDataHeader`.
/// Each vertex is represented by three floats (X, Y, Z).
#[derive(Debug, BinRead, BinWrite)]
#[br(big, import(count: u32))]
pub struct TriSurfVertices(#[br(count = count)] pub Vec<Point>);

impl TriSurfVertices {
    pub fn as_slice(&self) -> &[Point] {
        &self.0
    }
    pub fn iter(&self) -> impl Iterator<Item = &Point> {
        self.0.iter()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Triangle Array (`TRIS`).
///
/// Links the vertices from the `TriSurfVertices` chunk into a series of triangles.
/// Each triangle is represented by three unsigned integer vertex indices.
#[derive(Debug, BinRead, BinWrite)]
#[br(big, import(count: u32))]
pub struct TriSurfTriangles(#[br(count = count*3)] pub Vec<u32>);

/// Vertex Vector Array (`VVEC`).
///
/// Defines a vertex vector (also known as a vertex map) for a trisurf.
/// Multiple `VVEC` chunks can be defined for a single trisurf.
#[derive(Debug, BinWrite)]
pub struct TriSurfVertexVectors {
    /// Type of vector (e.g., `COLR` for color or `MORF` for morph).
    pub kind: ID4,
    /// Number of components in the vector.
    pub dimensions: u32,
    /// Name of the vector.
    #[bw(align_after = 2)]
    pub name: NullString,
    /// The actual vector data as an array of floats.
    pub vectors: Vec<f32>,
}

impl BinRead for TriSurfVertexVectors {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;

        let kind = ID4::read_be(reader)?;
        let dimensions = u32::read_be(reader)?;
        let name = read_aligned_nullstring(reader)?;

        let current_pos = reader.stream_position()?;
        let read_so_far = (current_pos - start) as u32;

        if read_so_far > size {
            return Err(binrw::Error::Custom {
                pos: start,
                err: Box::new(ParseError::InvalidSize),
            });
        }

        let bytes_left = size - read_so_far;
        if !bytes_left.is_multiple_of(4) {
            return Err(binrw::Error::Custom {
                pos: start,
                err: Box::new(ParseError::InvalidSize),
            });
        }

        let mut buf = vec![0u8; bytes_left as usize];
        reader.read_exact(&mut buf)?;

        let vectors: Vec<f32> = buf
            .chunks_exact(4)
            .map(|b| f32::from_be_bytes(b.try_into().unwrap()))
            .collect();

        Ok(TriSurfVertexVectors {
            kind,
            dimensions,
            name,
            vectors,
        })
    }
}

#[derive(Debug, BinWrite, BinRead, PartialEq, Eq)]
pub struct TriSurfTag {
    pub kind: ID4,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,
}

/// Tag Array (`TTGS`).
///
/// Defines one or more tags for a given trisurf as an array of type/value pairs.
#[derive(Debug, BinWrite)]
pub struct TriSurfTags(pub Vec<TriSurfTag>);

impl TriSurfTags {
    pub fn as_slice(&self) -> &[TriSurfTag] {
        &self.0
    }
    pub fn iter(&self) -> impl Iterator<Item = &TriSurfTag> {
        self.0.iter()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl BinRead for TriSurfTags {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let mut tags = vec![];
        while (reader.stream_position()? - start) < size as u64 {
            tags.push(TriSurfTag::read_be(reader)?);
        }

        Ok(TriSurfTags(tags))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkHeader;
    use binrw::BinReaderExt;
    use std::io::Cursor;
    use std::str::FromStr;

    #[test]
    fn trisurf_group_header() {
        let mut reader = Cursor::new([
            0x33, 0x47, 0x52, 0x50, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x23, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header: ChunkHeader = reader.read_be().unwrap();
        let group: TriSurfGroupHeader = reader.read_be().unwrap();

        assert_eq!(header.kind, "3GRP");
        assert_eq!(header.size, 12);
        assert_eq!(group.trisurf_count, 1);
        assert_eq!(group.item_reference, 35);
        assert_eq!(group.flags, 0);
    }

    #[test]
    fn trisurf_vertices_cube() {
        let mut reader = Cursor::new([
            0x56, 0x52, 0x54, 0x53, 0x00, 0x00, 0x00, 0x60, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00,
            0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00,
            0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00,
            0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00,
            0x00, 0x00, 0xbf, 0x00, 0x00, 0x00,
        ]);

        let header: ChunkHeader = reader.read_be().unwrap();
        let vertices = TriSurfVertices::read_args(&mut reader, (header.size / 12,)).unwrap();

        assert_eq!(header.kind, "VRTS");
        assert_eq!(header.size, 96);
        assert_eq!(vertices.len(), 8);
        assert_eq!(vertices.as_slice()[0], Point([-0.5, -0.5, 0.5]));
        assert_eq!(vertices.as_slice()[7], Point([0.5, 0.5, -0.5]));
    }

    #[test]
    fn trisurf_data_header() {
        let mut reader = Cursor::new([
            0x33, 0x53, 0x52, 0x46, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x01, 0x2a, 0x00, 0x00,
            0x01, 0xe0, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header: ChunkHeader = reader.read_be().unwrap();
        let data: TriSurfDataHeader = reader.read_be().unwrap();

        assert_eq!(header.kind, "3SRF");
        assert_eq!(header.size, 20);
        assert_eq!(data.vertex_count, 298);
        assert_eq!(data.triangle_count, 480);
        assert_eq!(data.vertex_vector_count, 4);
        assert_eq!(data.tag_count, 2);
        assert_eq!(data.flags, 0);
    }

    #[test]
    fn trisurf_tags_various_lengths() {
        // This is a synthetic test, so if we find a better true chunk to use replace this test
        let mut reader = Cursor::new([
            0x54, 0x54, 0x47, 0x53, 0x00, 0x00, 0x00, 0x14, 0x4D, 0x41, 0x54, 0x52, 0x00, 0x00,
            0x50, 0x41, 0x52, 0x54, 0x41, 0x00, 0x4D, 0x41, 0x54, 0x52, 0x41, 0x42, 0x00, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let tags = TriSurfTags::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(tags.len(), 3);

        assert_eq!(
            tags.as_slice(),
            vec![
                TriSurfTag {
                    kind: ID4::from_str("MATR").unwrap(),
                    name: "".into()
                },
                TriSurfTag {
                    kind: ID4::from_str("PART").unwrap(),
                    name: "A".into()
                },
                TriSurfTag {
                    kind: ID4::from_str("MATR").unwrap(),
                    name: "AB".into()
                },
            ]
        );

        let mut writer = Cursor::new(vec![]);
        header.write_be(&mut writer).unwrap();
        tags.write_be(&mut writer).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn unnamed_material_and_part_trisurf_tags() {
        let mut reader = Cursor::new([
            0x54, 0x54, 0x47, 0x53, 0x00, 0x00, 0x00, 0x0c, 0x4d, 0x41, 0x54, 0x52, 0x00, 0x00,
            0x50, 0x41, 0x52, 0x54, 0x00, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let tags = TriSurfTags::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(
            tags.as_slice(),
            vec![
                TriSurfTag {
                    kind: ID4::from_str("MATR").unwrap(),
                    name: "".into()
                },
                TriSurfTag {
                    kind: ID4::from_str("PART").unwrap(),
                    name: "".into()
                },
            ]
        );

        let mut writer = Cursor::new(vec![]);
        header.write_be(&mut writer).unwrap();
        tags.write_be(&mut writer).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }
}
