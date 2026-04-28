use bitflags::bitflags;
use binrw::{BinRead, NullString};
use::std::fmt;

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
    #[br(pad_size_to = 2)]
    pub name: NullString,
    pub parent: u16,
    pub subdivision_level: f32, // oddly enough, Modo's UI only allow integers to set this
    pub curve_angle: f32,
    pub scale_pivot: [f32; 3],  // 40...
    pub unused: [u32; 6],       // 64
    pub reference: u32,         // 68
    pub spline_patch_level: u16,//70
    pub future_expansion: [u16; 3], // 76
    pub unknown: [u16; 7],  // todo: find what each u16 here means in Modo
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkHeader;
    use std::io::Cursor;
    use binrw::BinReaderExt;

    #[test]
    fn test_parse_layer_cube_lxo() {
        // $ xxd -s 5332 -l 98 tests/fixtures/cube.lxo
        // 000014d4: 4c41 5952 0000 005a 0000 0005 0000 0000  LAYR...Z........
        // 000014e4: 0000 0000 0000 0000 0000 ffff 4000 0000  ............@...
        // 000014f4: 3db2 b8c2 3f80 0000 0000 0000 0000 0000  =...?...........
        // 00001504: 0000 0000 3f80 0000 0000 0000 0000 0000  ....?...........
        // 00001514: 0000 0000 3f80 0000 0000 0000 0010 0000  ....?...........
        // 00001524: 0000 0000 0001 0002 0002 0002 0001 0001  ................
        // 00001534: 0000                                     ..
        let mut reader = Cursor::new([
            0x4c, 0x41, 0x59, 0x52, 0x00, 0x00, 0x00, 0x5a, 0x00, 0x00, 0x00, 0x05,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xff, 0xff, 0x40, 0x00, 0x00, 0x00, 0x3d, 0xb2, 0xb8, 0xc2,
            0x3f, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x00
        ]);

        let _: ChunkHeader = reader.read_be().unwrap();
        let layer: Layer = reader.read_be().unwrap();

        assert_eq!(layer.index, 0, "Failed to parse index");
        assert_eq!(layer.flags, LayerFlag::Default, "Failed to parse layer flag");
        assert!(layer.name.is_empty(), "Failed to parse name");
        assert_eq!(layer.parent, 0xffff, "Expected parent to be 0xffff, ie not set");
        assert_eq!(layer.subdivision_level, 2.0, "Expected 2.0 in subdivision level");
        assert_eq!(layer.reference, 0, "Reference bad");
        assert_eq!(layer.spline_patch_level, 16, "spline patch");
        assert_eq!(layer.future_expansion, [0, 0, 0]);

        // todo -  as we don't know what the other values can mean we leave them out from tests
    }

    #[test]
    fn test_parse_cube_points() {
        // $ xxd -s 5430 -l 104 tests/fixtures/cube.lxo
        // 00001536: 504e 5453 0000 0060 bf00 0000 bf00 0000  PNTS...`........
        // 00001546: 3f00 0000 3f00 0000 bf00 0000 3f00 0000  ?...?.......?...
        // 00001556: 3f00 0000 bf00 0000 bf00 0000 bf00 0000  ?...............
        // 00001566: bf00 0000 bf00 0000 bf00 0000 3f00 0000  ............?...
        // 00001576: 3f00 0000 3f00 0000 3f00 0000 3f00 0000  ?...?...?...?...
        // 00001586: 3f00 0000 3f00 0000 bf00 0000 bf00 0000  ?...?...........
        // 00001596: 3f00 0000 bf00 0000                      ?.......
        let mut reader = Cursor::new([
            b'P', b'N', b'T', b'S', 0x00, 0x00, 0x00, 0x60,
            0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 
            0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 
            0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 
            0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 
            0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 
            0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00,
        ]);

        let header: ChunkHeader = reader.read_be().unwrap();
        let points = Points::read_args(&mut reader, (header.size / 12,)).unwrap();

        assert_eq!(points.0.len(), 8);
    }
}
