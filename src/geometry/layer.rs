use crate::primitives::{ID4, Point, VX, read_vx_from_bytes};
use crate::utils::read_aligned_nullstring;
use binrw::meta::{EndianKind, ReadEndian};
use binrw::{
    BinRead, BinResult, BinWrite, Endian, NullString,
    io::{Read, Seek, Write},
};
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::ops::Deref;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LayerFlag: u16 {
        const Visible           = 0b0000_0000_0000_0001;
        const Hidden            = 0b0000_0000_0000_0010;
        const Foreground        = 0b0000_0000_0000_0100;
        const Background        = 0b0000_0000_0000_1000;
        const Boundingbox       = 0b0000_0000_0001_0000;
        const LinearUv          = 0b0000_0000_1000_0000;
        const Multiresolution   = 0b0010_0000_0000_0000;
        const Default           = Self::Visible.bits() | Self::Foreground.bits();
    }
}

#[derive(BinRead, BinWrite, Debug, PartialEq, Eq)]
#[br(repr=u16)]
#[bw(repr=u16)]
pub enum BoundaryRules {
    SmoothAll,
    CreaseAll,
    CreaseEdges,
}

// Optional data for layers, mostly unknown what the fields mean.
#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct Multiresolution {
    pub unk0: [u8; 8],
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,
    pub unk1: [u8; 34],
}

#[derive(Debug)]
pub struct PolygonGroup {
    pub polygons: PolygonList,
    pub vertex_maps: Vec<DiscontinousVertexMap>,
    pub tags: Vec<PolygonTagMapping>,
}

impl PolygonGroup {
    pub fn new(polygons: PolygonList) -> PolygonGroup {
        let vertex_maps = vec![];
        let tags = vec![];
        PolygonGroup {
            polygons,
            vertex_maps,
            tags,
        }
    }
}

#[derive(Debug, Default)]
pub struct LayerGeometry {
    pub points: Option<Points>,
    pub bounds: Option<BoundingBox>,
    pub vertex_map_parameters: Vec<VertexMapParameter>,
    pub vertex_maps: Vec<VertexMap>,
    pub vertex_edge_maps: Vec<VertexEdgeMap>,
    pub polygons: BTreeMap<ID4, PolygonGroup>,
}

#[derive(BinRead, BinWrite, Debug, PartialEq, Eq)]
#[br(repr=u16)]
#[bw(repr=u16)]
pub enum Smoothing {
    AlwaysEnabled,
    DisabledWithDeformers,
    AlwaysDisabled,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct Layer {
    pub index: u16,
    #[br(map = |x: u16| LayerFlag::from_bits_retain(x))]
    #[bw(map = |x: &LayerFlag| x.bits())]
    pub flags: LayerFlag,
    pub pivot: [f32; 3],
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,
    pub parent: u16,
    pub subdivision_level: f32,
    pub curve_angle: f32,

    /// 3x3 Matrix for scale and rotation (I think, only seen as identity in all samples )
    pub matrix: [f32; 9],

    // Documentation says we have a scale pivot f32;3, and 6 unused u32, but they all smell like
    // floats. So can only assume they should be combined into one 3x3 matrix
    // pub scale_pivot: [f32; 3],
    // pub unused: [u32; 6],
    pub reference: u32,
    pub spline_patch_level: u16,
    pub future_expansion: [u16; 3],
    pub boundary_rules: BoundaryRules,
    pub catmull_render_level: u16,
    pub catmull_subdivision_level: u16,
    pub subdivision_render_level: u16,
    #[br(map = |x: u16| x != 0)]
    #[bw(map = |x: &bool| u16::from(*x))]
    pub cache_normal_vectors: bool,
    pub catmull_current_level: u16,
    pub smoothing: Smoothing,

    #[br(if(flags.contains(LayerFlag::Multiresolution)))]
    pub multires: Option<Multiresolution>,

    #[br(ignore)]
    #[bw(ignore)]
    pub geometry: LayerGeometry,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big, import(count: u32))]
#[bw(big)]
pub struct Points(#[br( count = count )] pub Vec<Point>);

impl Deref for Points {
    type Target = Vec<Point>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct BoundingBox {
    pub min: Point,
    pub max: Point,
}

#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(repr=i32)]
#[bw(repr=i32)]
pub enum UvSubdivisionKind {
    Linear,
    Subpatch,
    SubpatchLinearCorners,
    SubpatchLinearEdges,
    SubpatchDiscoEdges,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
pub struct VertexMapParameter {
    pub uv_subdivision: UvSubdivisionKind,
    pub sketch_color: i32,
}

#[derive(BinWrite, Debug)]
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

#[derive(BinWrite, Debug)]
pub struct VertexEdgeMap {
    pub kind: ID4,
    pub dimension: u16,
    pub name: NullString,
    pub data: Vec<(VX, VX, Vec<f32>)>,
}

impl BinRead for VertexEdgeMap {
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
            let vertex_a = VX::read_be(reader)?;
            let vertex_b = VX::read_be(reader)?;
            let mut values: Vec<f32> = vec![0.0; dimension as usize];
            for value in values.iter_mut() {
                *value = f32::read_be(reader)?;
            }

            data.push((vertex_a, vertex_b, values));
        }

        Ok(VertexEdgeMap {
            kind,
            dimension,
            name,
            data,
        })
    }
}

#[derive(BinWrite, Debug)]
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

#[derive(BinWrite, Debug)]
pub struct Polygon {
    pub vertex_count: u16,
    pub vertex_index: Vec<VX>,
    pub flags: Option<u32>,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
    pub struct CurveHandle: u8 {
        const Start     = 0x1;
        const End       = 0x2;
        const Closed    = 0x3;
    }
}

fn read_polygon_from_bytes(buf: &[u8], kind: ID4) -> Result<(Polygon, usize), binrw::Error> {
    if buf.len() < 2 {
        return Err(binrw::Error::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "polygon vertex_count truncated",
        )));
    }
    let vertex_count = u16::from_be_bytes([buf[0], buf[1]]);
    let mut offset = 2usize;

    let is_curve = kind == "HCRV" || kind == "BCRV" || kind == "BSPL";
    let flags = if is_curve {
        if buf.len() < offset + 4 {
            return Err(binrw::Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "polygon flags truncated",
            )));
        }
        let f = u32::from_be_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]);
        offset += 4;
        Some(f)
    } else {
        None
    };

    let mut vertex_index = Vec::with_capacity(vertex_count as usize);
    for _ in 0..vertex_count {
        let (vx, consumed) = read_vx_from_bytes(&buf[offset..])?;
        vertex_index.push(vx);
        offset += consumed;
    }

    Ok((
        Polygon {
            vertex_count,
            vertex_index,
            flags,
        },
        offset,
    ))
}

/// Each polygon is defined by a vertex count followed by a list of indices into the most recent PNTS chunk.
///
/// For writing POLS, the vertex list for each polygon should begin at a convex vertex and proceed clockwise
/// as seen from the visible side. Polygons are single-sided (double-sidedness is a possible surface property),
/// and the normal is defined as the cross product of the first and last edges.
///
/// Starting from version 701, HCRV & BCRV chunks, and from version 801, the LINE chunk,
/// utilize a separate 'flags' U4, which allows the vertex count to be represented by a full U2.
#[derive(BinWrite, Debug)]
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
        let mut buf = vec![0u8; size as usize];
        reader.read_exact(&mut buf)?;

        let kind = ID4::from_bytes([buf[0], buf[1], buf[2], buf[3]]).map_err(|e| {
            binrw::Error::Custom {
                pos: 0,
                err: Box::new(e),
            }
        })?;

        let mut polygons = Vec::new();
        let mut offset = 4usize;
        while offset < buf.len() {
            let (poly, consumed) = read_polygon_from_bytes(&buf[offset..], kind)?;
            polygons.push(poly);
            offset += consumed;
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

impl BinWrite for PolygonTagMapping {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<()> {
        self.kind.write_options(writer, _endian, ())?;
        for (poly, tag) in &self.tags {
            poly.write_options(writer, _endian, ())?;
            tag.write_options(writer, _endian, ())?;
        }

        Ok(())
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

        let header: ChunkHeader = reader.read_be().unwrap();
        let layer: Layer = reader.read_be().unwrap();

        assert_eq!(layer.index, 0, "Failed to parse index");
        assert_eq!(layer.flags, LayerFlag::Default);
        assert!(layer.name.is_empty());
        assert_eq!(layer.parent, 0xffff);
        assert_eq!(layer.subdivision_level, 2.0);
        assert_eq!(layer.reference, 0);
        assert_eq!(layer.spline_patch_level, 16);
        assert_eq!(layer.future_expansion, [0u16; 3]);
        assert_eq!(layer.boundary_rules, BoundaryRules::CreaseAll);

        assert!(layer.multires.is_none());

        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
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
            0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x66, 0x42, 0x43, 0x52, 0x56, 0x00, 0x2e,
            0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04,
            0x00, 0x05, 0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00, 0x09, 0x00, 0x0a, 0x00, 0x0b,
            0x00, 0x0c, 0x00, 0x0d, 0x00, 0x0e, 0x00, 0x0f, 0x00, 0x10, 0x00, 0x11, 0x00, 0x12,
            0x00, 0x13, 0x00, 0x14, 0x00, 0x15, 0x00, 0x16, 0x00, 0x17, 0x00, 0x18, 0x00, 0x19,
            0x00, 0x1a, 0x00, 0x1b, 0x00, 0x1c, 0x00, 0x1d, 0x00, 0x1e, 0x00, 0x1f, 0x00, 0x20,
            0x00, 0x21, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24, 0x00, 0x25, 0x00, 0x26, 0x00, 0x27,
            0x00, 0x28, 0x00, 0x29, 0x00, 0x2a, 0x00, 0x2b, 0x00, 0x2c, 0x00, 0x2d,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let pols = PolygonList::read_args(&mut reader, header.size).unwrap();

        assert_eq!(pols.kind, "BCRV");
        assert_eq!(pols.polygons.len(), 1);
        assert_eq!(pols.polygons[0].vertex_count, 46);
        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn named_multiresolution_layer() {
        let mut reader = Cursor::new([
            0x4c, 0x41, 0x59, 0x52, 0x00, 0x00, 0x00, 0x9a, 0x00, 0x02, 0x20, 0x05, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4d, 0x75, 0x6c, 0x74,
            0x69, 0x72, 0x65, 0x73, 0x6f, 0x6c, 0x75, 0x74, 0x69, 0x6f, 0x6e, 0x00, 0xff, 0xff,
            0x40, 0x00, 0x00, 0x00, 0x3e, 0x86, 0x0a, 0x92, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x4c, 0x61, 0x79, 0x65, 0x72, 0x20,
            0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
            0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let layer = Layer::read_be(&mut reader).unwrap();

        assert_eq!(layer.index, 2);
        assert_eq!(layer.flags, LayerFlag::Default | LayerFlag::Multiresolution);
        assert_eq!(layer.pivot, [0f32; 3]);
        assert_eq!(layer.name, "Multiresolution".into());
        assert_eq!(layer.parent, 0xffff);
        assert_eq!(layer.subdivision_level, 2f32);
        assert_eq!(layer.spline_patch_level, 3);
        assert_eq!(layer.curve_angle, 15f32.to_radians());

        assert!(layer.multires.is_some());

        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn nameless_multiresolution_layer() {
        let mut reader = Cursor::new([
            0x4c, 0x41, 0x59, 0x52, 0x00, 0x00, 0x00, 0x8c, 0x00, 0x01, 0x20, 0x05, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff,
            0x40, 0x00, 0x00, 0x00, 0x3d, 0xb2, 0xb8, 0xc2, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x4c, 0x61, 0x79, 0x65, 0x72, 0x20,
            0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
            0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let layer = Layer::read_be(&mut reader).unwrap();

        assert_eq!(layer.index, 1);
        assert_eq!(layer.flags, LayerFlag::Default | LayerFlag::Multiresolution);
        assert_eq!(layer.pivot, [0f32; 3]);
        assert!(layer.name.is_empty());
        assert_eq!(layer.parent, 0xffff);
        assert_eq!(layer.subdivision_level, 2f32);

        assert!(layer.multires.is_some());

        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn pols_face_triangle() {
        // Created by making a flat cylinder, with 3 sides.
        let mut reader = Cursor::new([
            0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x0c, 0x46, 0x41, 0x43, 0x45, 0x00, 0x03,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x01,
        ]);
        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let pols = PolygonList::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(pols.kind, "FACE");
        assert_eq!(pols.polygons.len(), 1);
        assert_eq!(pols.polygons[0].vertex_count, 3);
        assert_eq!(
            pols.polygons[0].vertex_index,
            vec![VX::U2(0), VX::U2(2), VX::U2(1)]
        );

        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn pols_face_quad() {
        // Created by making a flat cube,
        let mut reader = Cursor::new([
            0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x0e, 0x46, 0x41, 0x43, 0x45, 0x00, 0x04,
            0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00, 0x01,
        ]);
        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let pols = PolygonList::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(pols.kind, "FACE");
        assert_eq!(pols.polygons.len(), 1);
        assert_eq!(pols.polygons[0].vertex_count, 4);
        assert_eq!(
            pols.polygons[0].vertex_index,
            vec![VX::U2(0), VX::U2(3), VX::U2(2), VX::U2(1)]
        );

        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn pols_curve_start() {
        // This is a POLS chunks, which represents a curve in Modo with four points.
        // Where we've set Start = True
        let mut reader = Cursor::new([
            0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x12, 0x48, 0x43, 0x52, 0x56, 0x00, 0x04,
            0x00, 0x00, 0x1a, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03,
        ]);

        let header = ChunkHeader::read_be(&mut reader).expect("Chunk header id POLS with size 18");
        let pols = PolygonList::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(pols.kind, "HCRV");
        assert_eq!(pols.polygons.len(), 1);
        assert_eq!(
            pols.polygons[0].flags.unwrap() as u8,
            CurveHandle::Start.bits()
        );

        // Check that we have read all bytes
        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn pols_curve_end() {
        // This is a POLS chunks, which represents a curve in Modo with four points.
        // Where we've set End = True
        let mut reader = Cursor::new([
            0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x12, 0x48, 0x43, 0x52, 0x56, 0x00, 0x04,
            0x00, 0x00, 0x1a, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03,
        ]);

        let header = ChunkHeader::read_be(&mut reader).expect("Chunk header id POLS with size 18");
        let pols = PolygonList::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(pols.kind, "HCRV");
        assert_eq!(pols.polygons.len(), 1);
        assert_eq!(
            pols.polygons[0].flags.unwrap() as u8,
            CurveHandle::End.bits()
        );

        // Check that we have read all bytes
        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn pols_curve_closed() {
        // This is a POLS chunks, which represents a curve in Modo with four points.
        // Where we've set Closed = True
        let mut reader = Cursor::new([
            0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x18, 0x48, 0x43, 0x52, 0x56, 0x00, 0x07,
            0x00, 0x00, 0x1a, 0x03, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03,
            0x00, 0x00, 0x00, 0x01,
        ]);

        let header = ChunkHeader::read_be(&mut reader).expect("Chunk header id POLS with size 18");
        let pols = PolygonList::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(pols.kind, "HCRV");
        assert_eq!(pols.polygons.len(), 1);
        assert_eq!(
            pols.polygons[0].flags.unwrap() as u8,
            CurveHandle::Closed.bits()
        );

        // Check that we have read all bytes
        assert_eq!(
            reader.stream_position().unwrap(),
            (header.size + 8).into(),
            "Failed to read the whole chunk"
        );
    }

    #[test]
    fn curve_closed_filled() {}

    #[test]
    fn text_pols_hello_world() {
        // This is hex dump from a layer and all it's data chunks where a text was created,
        // with Hello, World! as content with default settings. Font might differ depending
        // what is available on the system?
        let mut reader = Cursor::new([
            0x54, 0x41, 0x47, 0x53, 0x00, 0x00, 0x00, 0x5a, 0x4c, 0x42, 0x00, 0x00, 0x48, 0x65,
            0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64, 0x21, 0x00, 0x74, 0x65,
            0x78, 0x74, 0x00, 0x00, 0x40, 0x54, 0x3a, 0x38, 0x35, 0x31, 0x34, 0x6f, 0x65, 0x6d,
            0x20, 0x4e, 0x6f, 0x72, 0x6d, 0x61, 0x6c, 0x3a, 0x38, 0x35, 0x31, 0x34, 0x6f, 0x65,
            0x6d, 0x2c, 0x31, 0x30, 0x30, 0x30, 0x2c, 0x2d, 0x31, 0x2c, 0x35, 0x2c, 0x35, 0x30,
            0x2c, 0x30, 0x2c, 0x30, 0x2c, 0x30, 0x2c, 0x30, 0x2c, 0x30, 0x00, 0x00, 0x44, 0x65,
            0x66, 0x61, 0x75, 0x6c, 0x74, 0x00, 0x44, 0x65, 0x66, 0x61, 0x75, 0x6c, 0x74, 0x00,
            0x4c, 0x41, 0x59, 0x52, 0x00, 0x00, 0x00, 0x5a, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff,
            0x40, 0x00, 0x00, 0x00, 0x3d, 0xb2, 0xb8, 0xc2, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
            0x50, 0x4e, 0x54, 0x53, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x80, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00,
            0x00, 0x00, 0x42, 0x42, 0x4f, 0x58, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x3f, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x56, 0x4d, 0x50, 0x41, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x56, 0x4d, 0x41, 0x50, 0x00, 0x00,
            0x00, 0x10, 0x42, 0x53, 0x50, 0x4c, 0x00, 0x01, 0x42, 0x2d, 0x53, 0x70, 0x6c, 0x69,
            0x6e, 0x65, 0x00, 0x00, 0x50, 0x4f, 0x4c, 0x53, 0x00, 0x00, 0x00, 0x0c, 0x54, 0x45,
            0x58, 0x54, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x50, 0x54, 0x41, 0x47,
            0x00, 0x00, 0x00, 0x08, 0x4d, 0x41, 0x54, 0x52, 0x00, 0x00, 0x00, 0x05, 0x50, 0x54,
            0x41, 0x47, 0x00, 0x00, 0x00, 0x08, 0x50, 0x41, 0x52, 0x54, 0x00, 0x00, 0x00, 0x04,
            0x50, 0x54, 0x41, 0x47, 0x00, 0x00, 0x00, 0x08, 0x46, 0x4f, 0x4e, 0x54, 0x00, 0x00,
            0x00, 0x03, 0x50, 0x54, 0x41, 0x47, 0x00, 0x00, 0x00, 0x08, 0x4a, 0x55, 0x53, 0x54,
            0x00, 0x00, 0x00, 0x00, 0x50, 0x54, 0x41, 0x47, 0x00, 0x00, 0x00, 0x08, 0x54, 0x45,
            0x58, 0x54, 0x00, 0x00, 0x00, 0x01,
        ]);

        let tags_header = ChunkHeader::read_be(&mut reader).unwrap();
        let itags = crate::meta::ItemTags::read_be_args(&mut reader, tags_header.size).unwrap();

        assert_eq!(itags.len(), 6);
        assert_eq!(itags[0], "LB".into());
        assert_eq!(itags[1], "Hello, World!".into());
        assert_eq!(itags[2], "text".into());
        assert_eq!(
            itags[3],
            "@T:8514oem Normal:8514oem,1000,-1,5,50,0,0,0,0,0".into()
        );
        assert_eq!(itags[4], "Default".into());
        assert_eq!(itags[5], "Default".into());

        let _layer_header = ChunkHeader::read_be(&mut reader).unwrap();
        let _layer = Layer::read_be(&mut reader).unwrap();

        let points_header = ChunkHeader::read_be(&mut reader).unwrap();
        let points = Points::read_be_args(&mut reader, (points_header.size / 12,)).unwrap();
        assert_eq!(points.len(), 3);

        let _bounds_header = ChunkHeader::read_be(&mut reader).unwrap();
        let bounds = BoundingBox::read_be(&mut reader).unwrap();

        assert_eq!(bounds.min, [0.0, 0.0, 0.0].into());
        assert_eq!(bounds.max, [1.0, 1.0, 0.0].into());

        let _vmap_params_header = ChunkHeader::read_be(&mut reader).unwrap();
        let _vmap_params = VertexMapParameter::read_be(&mut reader).unwrap();

        let vmap_header = ChunkHeader::read_be(&mut reader).unwrap();
        let vmap = VertexMap::read_be_args(&mut reader, vmap_header.size).unwrap();

        assert_eq!(vmap.kind, "BSPL");
        assert_eq!(vmap.dimension, 1);
        assert_eq!(vmap.name, "B-Spline".into());
        assert!(vmap.data.is_empty());

        let polygons_header = ChunkHeader::read_be(&mut reader).unwrap();
        let polygons = PolygonList::read_be_args(&mut reader, polygons_header.size).unwrap();

        assert_eq!(polygons.kind, "TEXT");
        assert_eq!(polygons.polygons.len(), 1);

        let _ptag_header = ChunkHeader::read_be(&mut reader).unwrap();
        let tag0 = PolygonTagMapping::read_be_args(&mut reader, _ptag_header.size).unwrap();

        assert_eq!(tag0.kind, "MATR");
        assert_eq!(tag0.tags.get(&VX::U2(0)), Some(5u16).as_ref());

        let _ptag_header = ChunkHeader::read_be(&mut reader).unwrap();
        let tag1 = PolygonTagMapping::read_be_args(&mut reader, _ptag_header.size).unwrap();

        assert_eq!(tag1.kind, "PART");
        assert_eq!(tag1.tags.get(&VX::U2(0)), Some(4u16).as_ref());

        let _ptag_header = ChunkHeader::read_be(&mut reader).unwrap();
        let tag2 = PolygonTagMapping::read_be_args(&mut reader, _ptag_header.size).unwrap();

        assert_eq!(tag2.kind, "FONT");
        assert_eq!(tag2.tags.get(&VX::U2(0)), Some(3u16).as_ref());

        let _ptag_header = ChunkHeader::read_be(&mut reader).unwrap();
        let tag3 = PolygonTagMapping::read_be_args(&mut reader, _ptag_header.size).unwrap();

        assert_eq!(tag3.kind, "JUST");
        assert_eq!(tag3.tags.get(&VX::U2(0)), Some(0u16).as_ref());

        let _ptag_header = ChunkHeader::read_be(&mut reader).unwrap();
        let tag4 = PolygonTagMapping::read_be_args(&mut reader, _ptag_header.size).unwrap();

        assert_eq!(tag4.kind, "TEXT");
        assert_eq!(tag4.tags.get(&VX::U2(0)), Some(1u16).as_ref());
    }

    #[test]
    #[ignore]
    fn benchmark_pols_parsing() {
        use std::io::BufReader;
        use std::path::PathBuf;
        use std::time::Instant;

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/fixtures/pols_benchmark");

        let file = std::fs::File::open(&path).expect("pols_benchmark file not found");
        let size = file.metadata().unwrap().len() as u32;
        let mut reader = BufReader::new(file);

        let now = Instant::now();
        let pols = PolygonList::read_be_args(&mut reader, size).unwrap();
        let elapsed = now.elapsed();

        let total_vertices: usize = pols.polygons.iter().map(|p| p.vertex_index.len()).sum();

        println!(
            "POLS benchmark: {} ms, {} polygons, {} total vertices, kind={}",
            elapsed.as_millis(),
            pols.polygons.len(),
            total_vertices,
            pols.kind
        );

        assert_eq!(pols.kind, "FACE");
        assert_eq!(reader.stream_position().unwrap(), size as u64);
    }
}
