use crate::primitives::{ChannelValue, SubChunkHeader, VX};
use crate::utils::read_aligned_nullstring;
use binrw::{BinRead, BinResult, BinWrite, Endian, NullString};
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Seek, Write};

#[derive(BinRead, BinWrite, Debug, PartialEq, Eq)]
#[br(big, repr=u32)]
#[bw(big, repr=u32)]
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

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
    pub struct Slope: u16 {
            const Direct        = 0b0000_0000;
            const Automatic     = 0b0000_0001;
            const LinearIn      = 0b0000_0010;
            const LinearOut     = 0b0000_0100;
            const Flat          = 0b0000_1000;
            const AutoFlat      = 0b0001_0000;
            const Stepped       = 0b0010_0000;
            const SmoothFlat    = 0b0100_0000;
    }
}

#[derive(BinRead, BinWrite, Debug, PartialEq, Eq)]
#[br(big, repr=u16)]
#[bw(big, repr=u16)]
pub enum Weight {
    Manual,
    Automatic,
}

#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(big, magic = b"TANI")]
#[bw(big, magic = b"TANI")]
pub struct TangentIn {
    #[bw(map = |_: &u16| 0x10u16)]
    _size: u16,
    #[br(map = |x: u16| Slope::from_bits_retain(x))]
    #[bw(map = |x: &Slope| x.bits())]
    pub slope_type: Slope,
    pub weight_type: Weight,
    pub weight: f32,
    pub slope: f32,
    pub value: f32,
}

impl TangentIn {
    pub fn new(
        slope_type: Slope,
        weight_type: Weight,
        weight: f32,
        slope: f32,
        value: f32,
    ) -> TangentIn {
        TangentIn {
            _size: 0x10,
            slope_type,
            weight_type,
            weight,
            slope,
            value,
        }
    }
}

#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(big, magic = b"TANO")]
#[bw(big, magic = b"TANO")]
pub struct TangentOut {
    #[bw(map = |_: &u16| 0x14u16)]
    _size: u16,
    pub breaks: Break,
    #[br(map = |x: u16| Slope::from_bits_retain(x))]
    #[bw(map = |x: &Slope| x.bits())]
    pub slope_type: Slope,
    pub weight_type: Weight,
    pub weight: f32,
    pub slope: f32,
    pub value: f32,
}

impl TangentOut {
    pub fn new(
        breaks: Break,
        slope_type: Slope,
        weight_type: Weight,
        weight: f32,
        slope: f32,
        value: f32,
    ) -> TangentOut {
        TangentOut {
            _size: 0x14,
            breaks,
            slope_type,
            weight_type,
            weight,
            slope,
            value,
        }
    }
}

#[derive(BinRead, BinWrite, Debug, PartialEq, Eq)]
#[br(big, repr=u32)]
#[bw(big, repr=u32)]
pub enum Break {
    Value,
    Slope,
    Weight,
}

#[derive(BinRead, BinWrite, Debug, PartialEq, Eq)]
#[br(big, repr=u16)]
#[bw(big, repr=u16)]
pub enum Behaviour {
    Reset,
    Constant,
    Repeat,
    Oscillate,
    OffsetRepeat,
    Linear,
    None,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big, magic = b"PRE ")]
#[bw(big, magic = b"PRE ")]
pub struct Pre {
    #[bw(map = |_: &u16| 2u16)]
    _size: u16,
    pub behaviour: Behaviour,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big, magic = b"POST")]
#[bw(big, magic = b"POST")]
pub struct Post {
    #[bw(map = |_: &u16| 2u16)]
    _size: u16,
    pub behaviour: Behaviour,
}

#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(big, magic = b"KEY ")]
#[bw(big, magic = b"KEY ")]
pub struct Key {
    #[bw(map = |_: &u16| 8u16)]
    _size: u16,
    pub time: f32,
    pub value: f32,
}

impl Key {
    pub fn new(time: f32, value: f32) -> Key {
        Key {
            _size: 8,
            time,
            value,
        }
    }
}

///
/// The flags sub-sub-chunk contains client-specific flags for the keyframe. These are deprecated,
/// and are not used in any form in any version of modo. Any FLAG chunk found can simply be ignored.
///
#[derive(BinRead, BinWrite, Debug)]
#[br(big, magic = b"FLAG")]
#[bw(big, magic = b"FLAG")]
pub struct Flag {
    #[bw(map = |_: &u16| 4u16)]
    _size: u16,
    pub flag: u32,
}

///
/// # Envelope Chunk
///
/// The ENVL chunk describes an envelope applied to an item. In modo, envelopes define the keys of
/// gradients and for normal keyframed animation. Note that this is not the same as the LWO2
/// envelope chunk. The envelope contains three sub-chunks representing the spline, TANI, TANO
/// and KEY, as well as the behavior chunks PRE and POST.
///
/// The spline type used in modo is a variation on the bezier spline. The specific implementation
/// is not currently documented, but it should be close enough to standard bezier curves for you to
/// use that at the moment.
///
#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct Envelope {
    pub index: VX,
    pub kind: EnvelopeKind,
    pub pre: Pre,
    pub post: Post,
    pub key: Key,
    pub tangent_in: TangentIn,
    pub tangent_out: TangentOut,
    pub flag: Flag,
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
    pub items: BTreeMap<u32, Vec<ActionChannels>>,
}

#[derive(BinRead, BinWrite, Debug)]
pub struct ActionChannel {
    pub channel_index: VX,
    pub kind: u16,
    pub envelope_index: VX,
    #[br(args(kind))]
    pub variable: ChannelValue,
}

///
/// The CHNN sub-chunk contains information about a single channel's values for the preceding ITEM
/// sub-chunk. This is identical to the CHAN sub-chunk, but the channel is explicitly named
/// instead of using a lookup into the CHNM chunk's array.
///
#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct ActionNamedChannel {
    /// Name of the channel
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,

    /// Type of the channel
    pub kind: u16,

    /// Index of the envelope in the ENVL chunk's array, if applicable
    pub envelope_index: VX,

    /// Value of the channel. The datatype is determined by the kind field
    #[br(args(kind))]
    pub variable: ChannelValue,
}

///
/// The GRAD sub-chunk contains information about a single gradient channel's values for the
/// preceding ITEM sub-chunk.
///
#[derive(BinWrite, Debug)]
#[bw(big)]
pub struct ActionGradient {
    /// Index of the channel's name in the CHNM chunk's array
    pub channel_index: VX,

    /// Index of the envelope in the ENVL chunk's array, if applicable
    pub envelope_index: VX,

    pub flags: u32,

    /// Optional channel name.
    #[bw(align_after = 2)]
    pub name: Option<NullString>,

    #[bw(align_after = 2)]
    pub kind0: Option<NullString>,
    #[bw(align_after = 2)]
    pub kind1: Option<NullString>,
}

impl BinRead for ActionGradient {
    type Args<'a> = u16;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<ActionGradient> {
        let start = reader.stream_position()?;
        let channel_index = VX::read_be(reader)?;
        let envelope_index = VX::read_be(reader)?;
        let flags = u32::read_be(reader)?;

        let mut name = None;
        if reader.stream_position()? - start < size as u64 {
            name = Some(read_aligned_nullstring(reader)?);
        }

        let mut kind0 = None;
        if reader.stream_position()? - start < size as u64 {
            kind0 = Some(read_aligned_nullstring(reader)?);
        }

        let mut kind1 = None;
        if reader.stream_position()? - start < size as u64 {
            kind1 = Some(read_aligned_nullstring(reader)?);
        }

        Ok(ActionGradient {
            channel_index,
            envelope_index,
            flags,
            name,
            kind0,
            kind1,
        })
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

        let name = read_aligned_nullstring(reader)?;

        let kind = read_aligned_nullstring(reader)?;

        let reference = u32::read_options(reader, endian, ())?;

        let mut parent = None;
        let mut items: BTreeMap<u32, Vec<ActionChannels>> = BTreeMap::new();
        let mut current_item = 0;

        while reader.stream_position()? - start < size as u64 {
            let header = SubChunkHeader::read_be(reader)?;
            match header.kind.as_str() {
                "PRNT" => parent = Some(u32::read_be(reader)?),
                "ITEM" => {
                    // index into lxo items
                    current_item = u32::read_be(reader)?;
                    items.insert(current_item, Vec::new());
                }
                // These MUST come after ITEM subchunk,
                "CHAN" => {
                    let channel = ActionChannel::read_be(reader)?;
                    items
                        .entry(current_item)
                        .or_default()
                        .push(ActionChannels::CHAN(channel));
                }
                "CHNN" => {
                    let channel = ActionNamedChannel::read_be(reader)?;
                    items
                        .entry(current_item)
                        .or_default()
                        .push(ActionChannels::CHNN(channel));
                }
                "GRAD" => {
                    let channel = ActionGradient::read_be_args(reader, header.size)?;
                    items
                        .entry(current_item)
                        .or_default()
                        .push(ActionChannels::GRAD(channel));
                }
                "CHNS" => {
                    let channel = ActionString::read_be(reader)?;
                    items
                        .entry(current_item)
                        .or_default()
                        .push(ActionChannels::CHNS(channel));
                }
                _ => {
                    let pos = reader.stream_position()?;
                    reader.seek_relative(header.size as i64)?;
                    eprintln!(
                        "Unknown Action subchunk {} at {} size {}",
                        header.kind.as_str(),
                        pos - 6,
                        header.size
                    );
                }
            }
        }

        Ok(Action {
            name,
            kind,
            reference,
            parent,
            items,
        })
    }
}

impl BinWrite for Action {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<()> {
        self.name.write_be(writer)?;
        if !writer.stream_position()?.is_multiple_of(2) {
            0u8.write_be(writer)?;
        }

        self.kind.write_be(writer)?;
        if !writer.stream_position()?.is_multiple_of(2) {
            0u8.write_be(writer)?;
        }

        match self.parent {
            Some(_parent) => todo!("PRNT subchunk"),
            _ => {}
        }

        if !self.items.is_empty() {
            todo!("ITEM subchunk, with all it's 'child' channels");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkHeader;
    use binrw::BinWriterExt;
    use std::io::Cursor;

    #[test]
    fn test_parsing_envelope() {
        let mut reader = Cursor::new([
            0x45, 0x4e, 0x56, 0x4c, 0x00, 0x00, 0x00, 0x5e, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x50, 0x52, 0x45, 0x20, 0x00, 0x02, 0x00, 0x01, 0x50, 0x4f, 0x53, 0x54, 0x00, 0x02,
            0x00, 0x01, 0x4b, 0x45, 0x59, 0x20, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80,
            0x00, 0x00, 0x54, 0x41, 0x4e, 0x49, 0x00, 0x10, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x54, 0x41, 0x4e, 0x4f,
            0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46, 0x4c, 0x41, 0x47, 0x00, 0x04,
            0x00, 0x00, 0x0f, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let envelope = Envelope::read_be(&mut reader).unwrap();

        assert_eq!(envelope.index, VX::U2(2));
        assert_eq!(envelope.kind, EnvelopeKind::Float);
        assert_eq!(envelope.pre.behaviour, Behaviour::Constant);
        assert_eq!(envelope.post.behaviour, Behaviour::Constant);
        assert_eq!(envelope.key, Key::new(0.0, 1.0));
        assert_eq!(
            envelope.tangent_in,
            TangentIn::new(Slope::SmoothFlat, Weight::Manual, 0.0, 0.0, 1.0)
        );
        assert_eq!(
            envelope.tangent_out,
            TangentOut::new(
                Break::Value,
                Slope::SmoothFlat,
                Weight::Manual,
                0.0,
                0.0,
                0.0
            )
        );

        assert_eq!(reader.stream_position().unwrap(), (header.size + 8).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&envelope).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn scene_action() {
        let mut reader = Cursor::new([
            0x41, 0x43, 0x54, 0x4e, 0x00, 0x00, 0x00, 0x1a, 0x73, 0x63, 0x65, 0x6e, 0x65, 0x00,
            0x73, 0x63, 0x65, 0x6e, 0x65, 0x00, 0x00, 0x00, 0x00, 0x01, 0x50, 0x52, 0x4e, 0x54,
            0x00, 0x04, 0x00, 0x00, 0x00, 0x02,
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
    fn float_action_channel() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x0a, 0x01, 0xa5, 0x00, 0x02, 0x00, 0x00, 0x3f, 0x80,
            0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let channel = ActionChannel::read_be(&mut reader).unwrap();

        assert_eq!(channel.channel_index, VX::U2(421));
        assert_eq!(channel.kind, 2);
        assert_eq!(channel.envelope_index, VX::U2(0));
        assert_eq!(channel.variable, ChannelValue::Float(1.0));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());
    }

    #[test]
    fn string_action_channel() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x0e, 0x01, 0xd9, 0x00, 0x03, 0x00, 0x00, 0x66, 0x72,
            0x61, 0x6d, 0x65, 0x73, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = ActionChannel::read_be(&mut reader).unwrap();

        assert_eq!(chan.channel_index, VX::U2(473));
        assert_eq!(chan.kind, 3);
        assert_eq!(chan.envelope_index, VX::U2(0));
        assert_eq!(chan.variable, ChannelValue::String("frames".into()));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);

        writer.write_be(&header).unwrap();
        writer.write_be(&chan).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn nameless_action_gradient() {
        let mut reader = Cursor::new([
            0x47, 0x52, 0x41, 0x44, 0x00, 0x08, 0x01, 0x56, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let channel = ActionGradient::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(channel.channel_index, VX::U2(342));
        assert_eq!(channel.envelope_index, VX::U2(8));
        assert_eq!(channel.flags, 0);
        assert_eq!(channel.name, None);

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);

        writer.write_be(&header).unwrap();
        writer.write_be(&channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn named_action_gradient() {
        let mut reader = Cursor::new([
            0x47, 0x52, 0x41, 0x44, 0x00, 0x16, 0x00, 0x00, 0x00, 0x3a, 0x00, 0x00, 0x00, 0x00,
            0x4d, 0x79, 0x47, 0x72, 0x61, 0x64, 0x69, 0x65, 0x6e, 0x74, 0x2e, 0x42, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let channel = ActionGradient::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(channel.channel_index, VX::U2(0));
        assert_eq!(channel.envelope_index, VX::U2(58));
        assert_eq!(channel.flags, 0);
        assert_eq!(channel.name, Some("MyGradient.B".into()));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);

        writer.write_be(&header).unwrap();
        writer.write_be(&channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn curve_item_action_gradient() {
        let mut reader = Cursor::new([
            0x47, 0x52, 0x41, 0x44, 0x00, 0x16, 0x01, 0x23, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x66, 0x6c, 0x6f, 0x61, 0x74, 0x00, 0x66, 0x6c, 0x6f, 0x61, 0x74, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let channel = ActionGradient::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(channel.channel_index, VX::U2(291));
        assert_eq!(channel.envelope_index, VX::U2(20));
        assert_eq!(channel.flags, 0);
        assert_eq!(channel.name, Some("".into()));
        // Items which points to a layer with curves seem to be the only ones that have the extra
        // two NullString fields. And so far they have always contained "float"
        assert_eq!(channel.kind0, Some("float".into()));
        assert_eq!(channel.kind1, Some("float".into()));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);

        writer.write_be(&header).unwrap();
        writer.write_be(&channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }
}
