use crate::primitives::{ID4, Point, VX};
use crate::utils::read_aligned_nullstring;
use binrw::meta::{EndianKind, ReadEndian};
use binrw::{BinRead, BinResult, Endian, NullString};
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Seek};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LayerFlag: u16 {
        const Visible      = 0b0000_0001;
        const Hidden       = 0b0000_0010;
        const Foreground   = 0b0000_0100;
        const Background   = 0b0000_1000;
        const Boundingbox  = 0b0001_0000;
        const LinearUv     = 0b1000_0000;
        const Default      = Self::Visible.bits() | Self::Foreground.bits();
    }
}

#[derive(BinRead, Debug)]
#[br(repr=u16)]
pub enum BoundaryRules {
    SmoothAll,
    CreaseAll,
    CreaseEdges,
}

#[derive(BinRead, Debug)]
#[br(big)]
pub struct Layer {
    pub index: u16,
    #[br(map = |x: u16| LayerFlag::from_bits_retain(x))]
    pub flags: LayerFlag,
    pub pivot: [f32; 3],
    #[br(align_after = 2)]
    pub name: NullString,
    pub parent: u16,
    pub subdivision_level: f32,
    pub curve_angle: f32,
    pub scale_pivot: [f32; 3],
    pub unused: [u32; 6],
    pub reference: u32,
    pub spline_patch_level: u16,
    pub future_expansion: [u16; 3],
    pub boundary_rules: BoundaryRules,
    pub unknown: u16,
    pub multires: u16,

    // The following are actually separate chunks, but we are making the
    // layer be the owner as any PNTS that comes after a LAYR will belong
    // to that layer. And PNTS that appear before any LAYR will be discarded.

    #[br(ignore)]
    pub points: Option<Points>,

    #[br(ignore)]
    pub bounds: Option<BoundingBox>,

    #[br(ignore)]
    pub vertex_map_parameters: Vec<VertexMapParameter>,

    #[br(ignore)]
    pub vertex_maps: Vec<VertexMap>,

    #[br(ignore)]
    pub polygons: Vec<PolygonList>,

    #[br(ignore)]
    pub discontinous_vertex_maps: Vec<DiscontinousVertexMap>,

    #[br(ignore)]
    pub polygon_tags: Vec<PolygonTagMapping>,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.name.is_empty() {
            let _ = write!(f, "{}", self.index);
        }
        write!(f, "{}", self.name)
    }
}

#[derive(BinRead, Debug)]
#[br(big, import(count: u32))]
pub struct Points(#[br( count = count )] pub Vec<Point>);

#[derive(BinRead, Debug)]
#[br(big)]
pub struct BoundingBox {
    pub min: Point,
    pub max: Point,
}

#[derive(BinRead, Debug, PartialEq)]
#[br(repr=i32)]
pub enum UvSubdivisionKind {
    Linear,
    Subpatch,
    SubpatchLinearCorners,
    SubpatchLinearEdges,
    SubpatchDiscoEdges,
}

#[derive(BinRead, Debug)]
#[br(big)]
pub struct VertexMapParameter {
    pub uv_subdivision: UvSubdivisionKind,
    pub sketch_color: i32,
}

#[derive(Debug)]
pub struct VertexMap {
    pub kind: ID4,
    pub dimension: u16,
    pub name: NullString,
    pub data: Vec<(VX, Vec<f32>)>,
}

impl BinRead for VertexMap {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let kind = ID4::read_be(reader)?;
        let dimension = u16::read_be(reader)?;
        let name = read_aligned_nullstring(reader)?;

        let mut data = vec![];
        while reader.stream_position()? - start < size as u64 {
            let index = VX::read_be(reader)?;
            let mut values: Vec<f32> = vec![0.0; dimension as usize];
            for value in values.iter_mut() {
                *value = f32::read_be(reader)?;
            }

            data.push((index, values));
        }

        Ok(VertexMap {
            kind,
            dimension,
            name,
            data,
        })
    }
}

#[derive(Debug)]
pub struct DiscontinousVertexMap {
    pub kind: ID4,
    pub dimension: u16,
    pub name: NullString,
    pub data: Vec<(VX, VX, Vec<f32>)>,
}

impl BinRead for DiscontinousVertexMap {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let kind = ID4::read_be(reader)?;
        let dimension = u16::read_be(reader)?;
        let name = read_aligned_nullstring(reader)?;

        let mut data = vec![];
        while reader.stream_position()? - start < size as u64 {
            let vertex_index = VX::read_be(reader)?;
            let polygon_index = VX::read_be(reader)?;
            let mut values: Vec<f32> = vec![0.0; dimension as usize];
            for value in values.iter_mut() {
                *value = f32::read_be(reader)?;
            }

            data.push((vertex_index, polygon_index, values));
        }

        Ok(DiscontinousVertexMap {
            kind,
            dimension,
            name,
            data,
        })
    }
}

#[derive(Debug)]
pub struct Polygon {
    pub vertex_count: u16,
    pub vertex_index: Vec<VX>,
}

impl BinRead for Polygon {
    type Args<'a> = ID4;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        kind: Self::Args<'_>,
    ) -> BinResult<Self> {
        let raw = u16::read_be(reader)?;
        let vertex_count = if kind == "CURV" || kind == "BCRV" {
            // Bits 15-12: additional high count bits (extends to 14-bit max)
            // Bits 11-10: continuity toggle flags — 0 means handle IS present
            // Bits  9- 0: base vertex count
            let high  = (raw >> 12) & 0x000F;
            let low   = raw & 0x03FF;
            let base  = (high << 10) | low;

            // Each flag bit that is 0 means one continuity handle vertex is
            // stored in the VX list and must be read (0, 1, or 2 extra).
            let flags = (raw >> 10) & 0b11;
            let extra = (!flags & 0b01)         // bit 10 clear → start handle present
                      + ((!flags >> 1) & 0b01); // bit 11 clear → end handle present

            base + extra
        } else {
            // Standard polygons: top 6 bits are flags, bottom 10 are count.
            raw & 0x03FF
        };
        let mut vertex_index = Vec::with_capacity(vertex_count as usize);
        for _ in 0..vertex_count {
            vertex_index.push(VX::read_be(reader)?);
        }
        Ok(Polygon{vertex_count, vertex_index})
    }
}

#[derive(Debug)]
pub struct PolygonList {
    pub kind: ID4,
    pub polygons: Vec<Polygon>,
}

impl ReadEndian for PolygonList {
    const ENDIAN: EndianKind = EndianKind::Endian(Endian::Big);
}

impl BinRead for PolygonList {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        size: Self::Args<'_>,
    ) -> BinResult<PolygonList> {
        let start = reader.stream_position()?;
        let kind = ID4::read_be(reader)?;
        let mut polygons = vec![];
        while reader.stream_position()? - start < size as u64 {
            polygons.push(Polygon::read_be_args(reader, kind)?);
        }
        Ok(PolygonList { kind, polygons })
    }
}

#[derive(Debug)]
pub struct PolygonTagMapping {
    pub kind: ID4,
    pub tags: BTreeMap<VX, u16>,
}

impl BinRead for PolygonTagMapping {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let kind = ID4::read_be(reader)?;
        let mut tags = BTreeMap::new();
        while reader.stream_position()? - start < size as u64 {
            let poly = VX::read_be(reader)?;
            let tag = u16::read_be(reader)?;
            tags.insert(poly, tag);
        }
        Ok(PolygonTagMapping { kind, tags })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkHeader;
    use binrw::BinReaderExt;
    use std::io::Cursor;

    #[test]
    fn test_parse_layer_cube_lxo() {
        let mut reader = Cursor::new([
            0x4c, 0x41, 0x59, 0x52, 0x00, 0x00, 0x00, 0x5a, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff,
            0x40, 0x00, 0x00, 0x00, 0x3d, 0xb2, 0xb8, 0xc2, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
        ]);

        let _: ChunkHeader = reader.read_be().unwrap();
        let layer: Layer = reader.read_be().unwrap();

        assert_eq!(layer.index, 0, "Failed to parse index");
        assert_eq!(
            layer.flags,
            LayerFlag::Default,
            "Failed to parse layer flag"
        );
        assert!(layer.name.is_empty(), "Failed to parse name");
        assert_eq!(
            layer.parent, 0xffff,
            "Expected parent to be 0xffff, ie not set"
        );
        assert_eq!(
            layer.subdivision_level, 2.0,
            "Expected 2.0 in subdivision level"
        );
        assert_eq!(layer.reference, 0, "Reference bad");
        assert_eq!(layer.spline_patch_level, 16, "spline patch");
        assert_eq!(layer.future_expansion, [0, 0, 0]);

        // todo -  as we don't know what the other values can mean we leave them out from tests
    }

    #[test]
    fn test_parse_cube_points() {
        let mut reader = Cursor::new([
            b'P', b'N', b'T', b'S', 0x00, 0x00, 0x00, 0x60, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00,
            0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00,
            0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00,
            0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00,
            0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00,
            0x00, 0x00, 0xbf, 0x00, 0x00, 0x00,
        ]);

        let header: ChunkHeader = reader.read_be().unwrap();
        let points = Points::read_args(&mut reader, (header.size / 12,)).unwrap();

        assert_eq!(points.0.len(), 8);
    }

    #[test]
    fn test_parse_cube_polygons() {
        let mut reader = Cursor::new([
            0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x40, 0x46, 0x41, 0x43, 0x45, 0x00, 0x04,
            0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00, 0x01, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x05, 0x00, 0x04, 0x00, 0x05, 0x00, 0x01, 0x00, 0x02, 0x00, 0x06,
            0x00, 0x04, 0x00, 0x06, 0x00, 0x02, 0x00, 0x03, 0x00, 0x07, 0x00, 0x04, 0x00, 0x07,
            0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x04, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06,
            0x00, 0x07,
        ]);

        let header: ChunkHeader = reader.read_be().unwrap();
        let polygon_list = PolygonList::read_args(&mut reader, header.size).unwrap();

        assert_eq!(polygon_list.kind, "FACE");
        assert!(
            polygon_list
                .polygons
                .iter()
                .all(|polygon| polygon.vertex_count == 4)
        );
    }

    #[test]
    fn cube_bounding_box() {
        let mut reader = Cursor::new([
            0x42, 0x42, 0x4f, 0x58, 0x00, 0x00, 0x00, 0x18, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00,
            0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00,
        ]);

        let _ = ChunkHeader::read_be(&mut reader).unwrap();
        let bounds = BoundingBox::read_be(&mut reader).unwrap();

        assert_eq!(bounds.min, [-0.5, -0.5, -0.5].into());
        assert_eq!(bounds.max, [0.5, 0.5, 0.5].into());
    }

    #[test]
    fn cube_vertex_map_parameters() {
        let mut reader = Cursor::new([0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06]);
        let vmpa = VertexMapParameter::read_be(&mut reader).unwrap();

        assert_eq!(vmpa.uv_subdivision, UvSubdivisionKind::Subpatch);
        assert_eq!(vmpa.sketch_color, 6);
    }

    #[test]
    fn cube_vertex_map() {
        let mut reader = Cursor::new([
            0x56, 0x4d, 0x41, 0x50, 0x00, 0x00, 0x00, 0x5e, 0x54, 0x58, 0x55, 0x56, 0x00, 0x02,
            0x54, 0x65, 0x78, 0x74, 0x75, 0x72, 0x65, 0x00, 0x00, 0x07, 0x3e, 0x80, 0x00, 0x00,
            0x3f, 0x2a, 0xaa, 0xab, 0x00, 0x06, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x2a, 0xaa, 0xab,
            0x00, 0x05, 0x3f, 0x40, 0x00, 0x00, 0x3f, 0x2a, 0xaa, 0xab, 0x00, 0x04, 0x3f, 0x80,
            0x00, 0x00, 0x3f, 0x2a, 0xaa, 0xab, 0x00, 0x03, 0x3e, 0x80, 0x00, 0x00, 0x3e, 0xaa,
            0xaa, 0xab, 0x00, 0x02, 0x3f, 0x00, 0x00, 0x00, 0x3e, 0xaa, 0xaa, 0xab, 0x00, 0x01,
            0x3f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3e, 0x80, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);
        let header: ChunkHeader = reader.read_be().unwrap();
        let vmap = VertexMap::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(vmap.kind, "TXUV");
        assert_eq!(vmap.dimension, 2);
        assert_eq!(vmap.name, "Texture".into());
        assert_eq!(vmap.data.len(), 8);
    }

    #[test]
    fn cube_discontinous_vertex_map() {
        let mut reader = Cursor::new([
            0x56, 0x4d, 0x41, 0x44, 0x00, 0x00, 0x00, 0x62, 0x54, 0x58, 0x55, 0x56, 0x00, 0x02,
            0x54, 0x65, 0x78, 0x74, 0x75, 0x72, 0x65, 0x00, 0x00, 0x01, 0x00, 0x01, 0x3f, 0x40,
            0x00, 0x00, 0x3e, 0xaa, 0xaa, 0xab, 0x00, 0x00, 0x00, 0x01, 0x3f, 0x80, 0x00, 0x00,
            0x3e, 0xaa, 0xaa, 0xab, 0x00, 0x01, 0x00, 0x02, 0x3f, 0x40, 0x00, 0x00, 0x3e, 0xaa,
            0xaa, 0xab, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x2a, 0xaa, 0xab,
            0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x3e, 0xaa, 0xaa, 0xab, 0x00, 0x04,
            0x00, 0x05, 0x3e, 0x80, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x05, 0x00, 0x05,
            0x3f, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
        ]);

        let header: ChunkHeader = reader.read_be().unwrap();
        let _vmap = DiscontinousVertexMap::read_be_args(&mut reader, header.size).unwrap();
        assert_eq!(
            reader.stream_position().unwrap(),
            106,
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn cube_matr_ptags() {
        let mut reader = Cursor::new([
            0x50, 0x54, 0x41, 0x47, 0x00, 0x00, 0x00, 0x1c, 0x4d, 0x41, 0x54, 0x52, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x01,
            0x00, 0x04, 0x00, 0x01, 0x00, 0x05, 0x00, 0x01,
        ]);
        let header: ChunkHeader = reader.read_be().unwrap();
        let ptags = PolygonTagMapping::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(ptags.tags.len(), 6);
        assert!(ptags.tags.iter().all(|(_, v)| *v == 1));
    }

    #[test]
    fn bezier_curve_polygons() {
        let mut reader = Cursor::new([
            0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x66, 0x42, 0x43, 0x52, 0x56,
            0x00, 0x2e, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00, 0x07, 0x00, 0x08,
            0x00, 0x09, 0x00, 0x0a, 0x00, 0x0b, 0x00, 0x0c, 0x00, 0x0d, 0x00, 0x0e,
            0x00, 0x0f, 0x00, 0x10, 0x00, 0x11, 0x00, 0x12, 0x00, 0x13, 0x00, 0x14,
            0x00, 0x15, 0x00, 0x16, 0x00, 0x17, 0x00, 0x18, 0x00, 0x19, 0x00, 0x1a,
            0x00, 0x1b, 0x00, 0x1c, 0x00, 0x1d, 0x00, 0x1e, 0x00, 0x1f, 0x00, 0x20,
            0x00, 0x21, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24, 0x00, 0x25, 0x00, 0x26,
            0x00, 0x27, 0x00, 0x28, 0x00, 0x29, 0x00, 0x2a, 0x00, 0x2b, 0x00, 0x2c,
            0x00, 0x2d
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let pols = PolygonList::read_args(&mut reader, header.size).unwrap();

        assert_eq!(pols.kind, "BCRV");
        assert_eq!(pols.polygons.len(), 1);
        assert_eq!(pols.polygons[0].vertex_count, 48);
        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    #[ignore]
    fn multiresolution_layer() {
        // Binary dump from an empty layer, with Multiresolution set to True
        let mut reader = Cursor::new([
            0x4c, 0x41, 0x59, 0x52, 0x00, 0x00, 0x00, 0x8c, 0x00, 0x01, 0x20, 0x05,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xff, 0xff, 0x40, 0x00, 0x00, 0x00, 0x3d, 0xb2, 0xb8, 0xc2,
            0x3f, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x4c, 0x61,
            0x79, 0x65, 0x72, 0x20, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let layer = Layer::read_be(&mut reader).unwrap();

        assert_eq!(layer.index, 1);
        assert_eq!(layer.flags, LayerFlag::Default);
        assert_eq!(layer.pivot, [0f32; 3]);
        assert!(layer.name.is_empty());
        assert_eq!(layer.parent, 0xffff);
        assert_eq!(layer.subdivision_level, 2f32);

        assert_eq!(layer.multires, 0);

        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }
}
