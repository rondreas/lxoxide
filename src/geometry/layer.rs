use ::std::fmt;
use binrw::meta::{EndianKind, ReadEndian};
use binrw::{BinRead, BinResult, Endian, NullString};
use bitflags::bitflags;
use std::io::{Read, Seek};

use crate::ID4;
use crate::primitives::VX;

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

#[derive(BinRead, Debug, PartialEq)]
#[br(big)]
pub struct Layer {
    pub index: u16,
    #[br(map = |x: u16| LayerFlag::from_bits_retain(x))]
    pub flags: LayerFlag,
    pub pivot: [f32; 3],
    #[br(align_after = 2)]
    pub name: NullString,
    pub parent: u16,
    pub subdivision_level: f32, // oddly enough, Modo's UI only allow integers to set this
    pub curve_angle: f32,
    pub scale_pivot: [f32; 3],      // 40...
    pub unused: [u32; 6],           // 64
    pub reference: u32,             // 68
    pub spline_patch_level: u16,    //70
    pub future_expansion: [u16; 3], // 76
    pub unknown: [u16; 7],          // todo: find what each u16 here means in Modo
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
#[br(big)]
pub struct Point(pub [f32; 3]);

impl Point {
    pub fn x(&self) -> f32 {
        self.0[0]
    }

    pub fn y(&self) -> f32 {
        self.0[1]
    }

    pub fn z(&self) -> f32 {
        self.0[2]
    }
}

#[derive(BinRead, Debug)]
#[br(big, import(count: u32))]
pub struct Points(#[br( count = count )] pub Vec<Point>);

#[derive(BinRead, Debug)]
#[br(big)]
pub struct Polygon {
    pub vertex_count: u16,
    #[br( count = vertex_count )]
    pub vertex_index: Vec<VX>,
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
            polygons.push(Polygon::read_be(reader)?);
        }
        Ok(PolygonList { kind, polygons })
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
}
