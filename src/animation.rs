use binrw::BinRead;
use std::fmt;
use crate::primitives::VX;

// todo: many subchunks here have a fixed size, so _size should be replace with some method to
// 'skip' N bytes instead so we don't have a field for this in our structs. Maybe pad_before?

#[derive(BinRead, Debug)]
#[br(big, repr=u32)]
pub enum EnvelopeKind {
    Float,
    Integer
}

impl fmt::Display for EnvelopeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvelopeKind::Float => write!(f, "float"),
            EnvelopeKind::Integer => write!(f, "integer"),
        }
    }
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"TANI")]
pub struct TangentIn {
    _size: u16,
    pub slope_type: u16,
    pub weight_type: u16,
    pub weight: f32,
    pub slope: f32,
    pub value: f32,
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"TANO")]
pub struct TangentOut {
    _size: u16,
    pub breaks: u32,
    pub slope_type: u16,
    pub weight_type: u16,
    pub weight: f32,
    pub slope: f32,
    pub value: f32,
}

#[derive(BinRead, Debug)]
#[br(big, repr=u16)]
pub enum Behaviour {
    Reset,
    Constant,
    Repeat,
    Oscillate,
    OffsetRepeat,
    Linear,
    None,
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"PRE ")]
pub struct Pre{
    _size: u16,
    pub behaviour: Behaviour
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"POST")]
pub struct Post{
    _size: u16,
    pub behaviour: Behaviour
}


#[derive(BinRead, Debug)]
#[br(big, magic = b"KEY ")]
pub struct Key{
    _size: u16,
    pub time: f32,
    pub value: f32,
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"FLAG")]
pub struct Flag {
    _size: u16,
    pub flag: u32,
}

#[derive(BinRead, Debug)]
#[br(big)]
pub struct Envelope {
    pub index: VX,
    pub kind: EnvelopeKind,
    pub pre_behaviour: Pre,
    pub post_behaviour: Post,
    pub key: Key,
    pub tangent_in: TangentIn,
    pub tangent_out: TangentOut,
    pub flag: Flag,  // note: deprecated.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkHeader;
    use std::io::Cursor;

    #[test]
    fn test_parsing_envelope() {
        let mut reader = Cursor::new([
            0x45, 0x4e, 0x56, 0x4c, 0x00, 0x00, 0x00, 0x5e, // header
            0x00, 0x02, // variable sized index,
            0x00, 0x00, 0x00, 0x00,
            0x50, 0x52, 0x45, 0x20, 0x00, 0x02, 0x00, 0x01, // PRE 2 bytes in size, 1 in value 
            0x50, 0x4f, 0x53, 0x54, 0x00, 0x02, 0x00, 0x01, // POST, same as above.
            0x4b, 0x45, 0x59, 0x20, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x54, 0x41, 0x4e, 0x49, 0x00, 0x10,
            0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 
            0x54, 0x41, 0x4e, 0x4f, 0x00, 0x14,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x46, 0x4c, 0x41, 0x47, 0x00, 0x04, 0x00, 0x00, 0x0f, 0x00
        ]);

        let _ = ChunkHeader::read_be(&mut reader).unwrap();
        let envelope = Envelope::read_be(&mut reader).unwrap();
    }
}
