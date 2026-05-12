use crate::primitives::{ID4, Point};
use crate::utils::read_aligned_nullstring;
use binrw::{BinRead, BinResult, Endian, NullString};
use std::io::{Read, Seek};

#[derive(Debug, BinRead)]
pub struct TriSurfGroupHeader {
    pub trisurf_count: u32,
    pub item_reference: u32,
    pub flags: u32,
}

#[derive(Debug, BinRead)]
pub struct TriSurfDataHeader {
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub vertex_vector_count: u32,
    pub tag_count: u32,
    pub flags: u32,
}

#[derive(Debug, BinRead)]
#[br(big, import(count: u32))]
pub struct TriSurfVertices(#[br(count = count)] pub Vec<Point>);

#[derive(Debug, BinRead)]
#[br(big, import(count: u32))]
pub struct TriSurfTriangles(#[br(count = count)] pub Vec<[u32; 3]>);

#[derive(Debug)]
pub struct TriSurfVertexVectors {
    pub kind: ID4,
    pub dimensions: u32,
    pub name: NullString,
    pub vectors: Vec<Vec<f32>>,
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

        let mut vectors = Vec::new();
        if reader.stream_position()? - start < size as u64 {
            let mut v = Vec::with_capacity(dimensions as usize);
            for _ in 0..dimensions {
                v.push(f32::read_be(reader)?);
            }
            vectors.push(v);
        }

        Ok(TriSurfVertexVectors {
            kind,
            dimensions,
            name,
            vectors,
        })
    }
}

#[derive(Debug)]
pub struct TriSurfTags(pub Vec<(ID4, NullString)>);

impl BinRead for TriSurfTags {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let mut tags = Vec::new();

        let start = reader.stream_position()?;
        if reader.stream_position()? - start < size as u64 {
            let kind = ID4::read_be(reader)?;
            let name = read_aligned_nullstring(reader)?;

            tags.push((kind, name));
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

    #[test]
    fn trisurf_group_header() {
        let mut reader = Cursor::new([
            0x33, 0x47, 0x52, 0x50, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00,
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
    fn trisurf_data_header() {
        let mut reader = Cursor::new([
            0x33, 0x53, 0x52, 0x46, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x01, 0x2a,
            0x00, 0x00, 0x01, 0xe0, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x00,
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

}
