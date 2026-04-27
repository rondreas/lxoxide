use std::convert::TryFrom;
use crate::ParseError;

#[derive(Debug, PartialEq)]
pub struct Layer {
    pub index: u16,
    pub flags: u16,
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

        let flags = u16::from_be_bytes([data[offset], data[offset + 1]]);
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
        for i in 0..6 {
            unused[i] = u32::from_be_bytes([
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_layer_cube_lxo() {
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
        data.resize(90, 0);

        let layer = Layer::try_from(data).unwrap();
        assert_eq!(layer.index, 0);
        assert_eq!(layer.flags, 5);
        assert_eq!(layer.name, Vec::<u8>::new());
        assert_eq!(layer.parent, 0xffff);
        assert_eq!(layer.subdivision_level, 2.0);
        assert_eq!(layer.reference, 16);
        assert_eq!(layer.spline_patch_level, 1);
        assert_eq!(layer.future_expansion, [2, 2, 2]);
        assert_eq!(layer.extra.len(), 90 - 76);
    }
}
