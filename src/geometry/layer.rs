use std::convert::TryFrom;
use crate::ParseError;
use bitflags::bitflags;

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

#[derive(Debug, PartialEq)]
pub struct Layer {
    pub index: u16,
    pub flags: LayerFlag,
    pub pivot: [f32; 3],
    pub name: Vec<u8>,
    pub parent: u16,
    pub subdivision_level: f32,
    pub curve_angle: f32,
    pub scale_pivot: [f32; 3],
    pub unused: [u32; 6],
    pub reference: u32,
    pub spline_patch_level: u16,
    pub future_expansion: [u16; 3],
    pub extra: Vec<u8>,
}

impl TryFrom<Vec<u8>> for Layer {
    type Error = ParseError;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        if data.len() < 74 {
            return Err(ParseError::BufferTooShort);
        }

        let mut offset = 0;

        let index = u16::from_be_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        // let flags = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let flags = LayerFlag::from_bits(u16::from_be_bytes([data[offset], data[offset + 1]])).unwrap();
        offset += 2;

        let pivot = [
            f32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]),
            f32::from_be_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]),
            f32::from_be_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]]),
        ];
        offset += 12;

        let mut name_end = offset;
        while name_end < data.len() && data[name_end] != 0 {
            name_end += 1;
        }
        if name_end >= data.len() {
            return Err(ParseError::MissingNullTerminator);
        }
        let name = data[offset..name_end].to_vec();
        let name_len = name_end - offset + 1;
        offset += name_len + (name_len % 2);

        if offset + 60 > data.len() {
            return Err(ParseError::BufferTooShort);
        }

        let parent = u16::from_be_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let subdivision_level = f32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        offset += 4;

        let curve_angle = f32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        offset += 4;

        let scale_pivot = [
            f32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]),
            f32::from_be_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]),
            f32::from_be_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]]),
        ];
        offset += 12;

        let mut unused = [0u32; 6];
        for x in &mut unused {
            *x = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
        }

        let reference = u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        offset += 4;

        let spline_patch_level = u16::from_be_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let future_expansion = [
            u16::from_be_bytes([data[offset], data[offset + 1]]),
            u16::from_be_bytes([data[offset + 2], data[offset + 3]]),
            u16::from_be_bytes([data[offset + 4], data[offset + 5]]),
        ];
        offset += 6;

        let extra = if offset < data.len() {
            data[offset..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Layer {
            index,
            flags,
            pivot,
            name,
            parent,
            subdivision_level,
            curve_angle,
            scale_pivot,
            unused,
            reference,
            spline_patch_level,
            future_expansion,
            extra,
        })
    }
}

pub struct Points(pub Vec<[f32; 3]>);

impl Points {
    pub fn from_bytes(data: &[u8]) -> Result<Self, ParseError> {
        let points = data.chunks_exact(12).map(|point| {
            let x = f32::from_be_bytes(point[ 0..4].try_into().unwrap());
            let y = f32::from_be_bytes(point[ 4..8].try_into().unwrap());
            let z = f32::from_be_bytes(point[8..12].try_into().unwrap());
            [x,y,z]
        }).collect();
        Ok(Points(points))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Bytes from cube.lxo LAYR chunk (excluding header)
        let mut data: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x05, // index=0, flags=5
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // pivot
            0x00, // name=""
            0x00, // padding
            0xff, 0xff, // parent=0xffff
            0x40, 0x00, 0x00, 0x00, // subdiv=2.0
            0x3d, 0xb2, 0xb8, 0xc2, // angle
            0x3f, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // scale_pivot [1,0,0]
            0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0, // unused (6 * 4 = 24 bytes)
            0x00, 0x00, 0x00, 0x10, // ref=16
            0x00, 0x01, // spline_patch=1
            0x00, 0x02, 0x00, 0x02, 0x00, 0x02, // future [2,2,2]
        ];
        // The xxd output showed size 0x5a (90). 
        data.resize(90, 0);  // TODO: Remove, I don't like this part. The bytes from dump should be
        // in vec..

        let layer = Layer::try_from(data).unwrap();
        assert_eq!(layer.index, 0);
        assert_eq!(layer.flags, LayerFlag::Default);
        assert_eq!(layer.name, Vec::<u8>::new());
        assert_eq!(layer.parent, 0xffff);
        assert_eq!(layer.subdivision_level, 2.0);
        assert_eq!(layer.reference, 16);
        assert_eq!(layer.spline_patch_level, 1);
        assert_eq!(layer.future_expansion, [2, 2, 2]);
        assert_eq!(layer.extra.len(), 90 - 76);
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
        let data: Vec<u8> = vec![
            0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 
            0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 
            0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 
            0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 
            0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 
            0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x00,
        ];

        let points = Points::from_bytes(&data).unwrap();

        assert_eq!(points.0.len(), 8);
    }
}
