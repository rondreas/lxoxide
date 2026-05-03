use crate::primitives::VX;
use binrw::{BinRead, BinResult, NullString};
use std::io::{Read, Seek};
use std::fmt;
use crate::item::{SubChunkHeader, ChannelValue};
use std::collections::HashMap;

// todo: many subchunks here have a fixed size, so _size should be replace with some method to
// 'skip' N bytes instead so we don't have a field for this in our structs. Maybe pad_before?

#[derive(BinRead, Debug)]
#[br(big, repr=u32)]
pub enum EnvelopeKind {
    Float,
    Integer,
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
pub struct Pre {
    _size: u16,
    pub behaviour: Behaviour,
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"POST")]
pub struct Post {
    _size: u16,
    pub behaviour: Behaviour,
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"KEY ")]
pub struct Key {
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
    pub flag: Flag, // note: deprecated.
}

// Breaking with how other chunks have been done. I've made the fields more ergonomic I think.
#[derive(Debug)]
pub struct Action {
    pub name: NullString,
    pub kind: NullString,
    pub reference: u32,
    // pub flags: u32, NOTE: Spec mentions this field, but it is missing from example scenes.
    //
    pub parent: Option<u32>,
    pub items: HashMap<u32, Vec<ActionChannels>>,

    // container for unknown chunks... so we don't loose data in roundtrips.
    pub unknowns: Vec<(SubChunkHeader, Vec<u8>)>,
}

#[derive(BinRead, Debug)]
pub struct ActionChannel {
    pub channel_index: VX,
    pub kind: u16,
    pub envelope_index: VX,
    #[br(args(kind))]
    pub variable: ChannelValue
}

#[derive(BinRead, Debug)]
pub struct ActionNamedChannel {
    #[br(align_after = 2)]
    pub channel_index: NullString,
    pub kind: u16,
    pub envelope_index: VX,
    #[br(args(kind))]
    pub variable: ChannelValue
}

#[derive(Debug)]
pub struct ActionGradient {
    pub channel_index: VX,
    pub envelope_index: VX,
    pub flags: u32,
    pub name: Option<NullString>
}

impl BinRead for ActionGradient {
    type Args<'a> = u16;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<ActionGradient> {
        let start = reader.stream_position()?;
        let channel_index = VX::read_be(reader)?;
        let envelope_index = VX::read_be(reader)?;
        let flags = u32::read_be(reader)?;

        let mut name = None;
        if reader.stream_position()? - start < size as u64 {
            name = Some(NullString::read_be(reader)?);
            if !reader.stream_position()?.is_multiple_of(2) {
                reader.seek_relative(1)?;
            }
        }

        Ok(ActionGradient{channel_index, envelope_index, flags, name})
    }
}

#[derive(BinRead, Debug)]
pub struct ActionString {
    #[br(align_after = 2)]
    pub name: NullString,
    pub channel_index: VX,
    #[br(align_after = 2)]
    pub value: NullString,
}

#[derive(Debug)]
pub enum ActionChannels {
    CHAN(ActionChannel),
    CHNN(ActionNamedChannel),
    GRAD(ActionGradient),
    CHNS(ActionString),
}

impl BinRead for Action {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Action> {
        let start = reader.stream_position()?;

        let name = NullString::read_options(reader, endian, ())?;
        if !reader.stream_position().unwrap().is_multiple_of(2) {
            reader.seek_relative(1)?;
        }

        let kind = NullString::read_options(reader, endian, ())?;
        if !reader.stream_position().unwrap().is_multiple_of(2) {
            reader.seek_relative(1)?;
        }
        
        let reference = u32::read_options(reader, endian, ())?;

        let mut parent = None;
        let mut items: HashMap<u32, Vec<ActionChannels>> = HashMap::new();
        let mut current_item = 0;

        let mut unknowns: Vec<(SubChunkHeader, Vec<u8>)> = Vec::new();

        while reader.stream_position()? - start < size as u64 {
            let header = SubChunkHeader::read_be(reader)?;
            eprintln!("{}", header.kind.as_str());
            match header.kind.as_str() {
                "PRNT" => parent = Some(u32::read_be(reader)?),
                "ITEM" => {
                    current_item = u32::read_be(reader)?;
                    items.insert(current_item, Vec::new());
                },
                // These MUST come after ITEM subchunk,
                "CHAN" => {
                    let channel = ActionChannel::read_be(reader)?;
                    items.entry(current_item)
                        .or_insert_with(|| vec![])
                        .push(ActionChannels::CHAN(channel));
                },
                "CHNN" => {
                    let channel = ActionNamedChannel::read_be(reader)?;
                    items.entry(current_item)
                        .or_insert_with(|| vec![])
                        .push(ActionChannels::CHNN(channel));
                },
                "GRAD" => {
                    let channel = ActionGradient::read_be(reader)?;
                    items.entry(current_item)
                        .or_insert_with(|| vec![])
                        .push(ActionChannels::GRAD(channel));
                },
                "CHNS" => {
                    let channel = ActionString::read_be(reader)?;
                    items.entry(current_item)
                        .or_insert_with(|| vec![])
                        .push(ActionChannels::CHNS(channel));
                },
                _ => {
                    let pos = reader.stream_position()?;
                    let mut unknown: Vec<u8> = Vec::with_capacity(header.size as usize);
                    reader.read_exact(&mut unknown)?;
                    eprintln!(
                        "Unknown Action subchunk {} at {} size {}",
                        header.kind.as_str(), pos - 6, header.size
                    );
                    unknowns.push((header, unknown));
                }
            }
        }

        Ok(Action{name, kind, reference, parent, items, unknowns})
    }
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
            0x00, 0x00, 0x00, 0x00, 0x50, 0x52, 0x45, 0x20, 0x00, 0x02, 0x00,
            0x01, // PRE 2 bytes in size, 1 in value
            0x50, 0x4f, 0x53, 0x54, 0x00, 0x02, 0x00, 0x01, // POST, same as above.
            0x4b, 0x45, 0x59, 0x20, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00,
            0x54, 0x41, 0x4e, 0x49, 0x00, 0x10, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x54, 0x41, 0x4e, 0x4f, 0x00, 0x14,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46, 0x4c, 0x41, 0x47, 0x00, 0x04, 0x00, 0x00,
            0x0f, 0x00,
        ]);

        let _ = ChunkHeader::read_be(&mut reader).unwrap();
        let envelope = Envelope::read_be(&mut reader).unwrap();
    }

    #[test]
    fn test_parsing_action() {
        let mut reader = Cursor::new([
          0x41, 0x43, 0x54, 0x4e, 0x00, 0x00, 0x00, 0x1a, 0x73, 0x63, 0x65, 0x6e,
          0x65, 0x00, 0x73, 0x63, 0x65, 0x6e, 0x65, 0x00, 0x00, 0x00, 0x00, 0x01,
          0x50, 0x52, 0x4e, 0x54, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let action = Action::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(action.name, "scene".into());
        assert_eq!(action.kind, "scene".into());
        assert_eq!(action.reference, 1);
        assert_eq!(action.parent, Some(2));
        assert!(action.items.is_empty());
    }

    #[test]
    fn test_parsing_action_channel() {
        let mut reader = Cursor::new(
            [0x01, 0xa5, 0x00, 0x02, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00]
        );
        let channel = ActionChannel::read_be(&mut reader).unwrap();
        assert_eq!(channel.channel_index, VX::U2(421));
        assert_eq!(channel.kind, 2);
        assert_eq!(channel.envelope_index, VX::U2(0));
        assert_eq!(channel.variable, ChannelValue::Float(1.0));
    }

    #[test]
    fn test_parsing_action_gradient_channel() {
        let mut reader = Cursor::new([0x01, 0x56, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00]);
        let channel = ActionGradient::read_be(&mut reader).unwrap();
        assert_eq!(channel.channel_index, VX::U2(342));
        assert_eq!(channel.envelope_index, VX::U2(8));
        assert_eq!(channel.flags, 0);
        assert_eq!(channel.name, None);
    }
}
