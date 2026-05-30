use crate::ParseError;
use crate::primitives::{ChannelValue, ID4, SubChunkHeader, VX};
use crate::utils::{read_aligned_nullstring, write_aligned_nullstring};
use binrw::meta::{EndianKind, ReadEndian};
use binrw::{
    BinRead, BinResult, BinWrite, Endian, NullString,
    io::{Cursor, Read, Seek, Write},
};
use bitflags::bitflags;
use std::str::FromStr;

///
/// The LAYR sub-chunk contains layer-specific features for the item. This consists of a layer
/// index, flag bits, and a wireframe/element color.
///
#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct Layer {
    /// Index of the layer in the Layer List
    pub index: u32,

    /// Flags describing layer-specific properties
    #[br(map = |x: u32| LayerVisibilityFlags::from_bits_retain(x))]
    #[bw(map = |x: &LayerVisibilityFlags| x.bits())]
    pub flags: LayerVisibilityFlags,

    /// Four-element array representing the RGBA element (wireframe) color in the UI
    pub color: [u8; 4],
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LayerVisibilityFlags: u32 {
        const Visible = 1;
        const Hidden = 1 << 1;
        const Foreground = 1 << 2;
        const Background = 1 << 3;
        const BoundingBox = 1 << 4;
        const LinearSubdivUv = 1 << 7;
    }
}

///
/// The XREF sub-chunk identifies an external reference item, and is only present if this item is 
/// indeed a reference itself.
///
#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct Reference {
    /// Index for the sub-scene in XREF chunk
    pub index: u32,

    /// Filename containing the source scene being referenced
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub path: NullString,

    /// Item identifier in the source scene
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub ident: NullString,
}

#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct Package {
    /// Package name that is used to add the package and load and save its state
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,

    /// Package data size in bytes. Note that zero is a valid size
    pub size: u32,

    /// The package's data stored as raw bytes
    #[br(count = size, align_after = 2)]
    #[bw(align_after = 2)]
    pub data: Vec<u8>,
}

#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
#[br(big)]
pub struct Channel {
    pub index: VX,
    pub kind: u16,
    #[br(args(kind))]
    pub value: ChannelValue,
}

///
/// The LINK sub-chunk relates one item to another item. Parenting is one kind of linking. 
/// LINK sub-chunks contain a graph type name, unique ID to the target item, and the index of the 
/// link. Zero or more of these may be present in an ITEM chunk.
///
#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
#[br(big)]
pub struct Link {
    /// The name of the graph that this link belongs to, such as parent
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,

    /// The ID of the item in the scene
    pub id: u32,

    /// The index of the link
    pub index: u32,
}

#[derive(Debug, BinWrite, Clone, PartialEq)]
pub struct Gradient {
    #[bw(align_after = 2)]
    pub name: NullString,
    pub envelope_index: VX,
    pub flags: EnvelopeInterpolationFlag,

    #[bw(align_after = 2)]
    pub kind0: Option<NullString>,

    #[bw(align_after = 2)]
    pub kind1: Option<NullString>,
}

impl BinRead for Gradient {
    type Args<'a> = u16;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let name = read_aligned_nullstring(reader)?;
        let envelope_index = VX::read_be(reader)?;
        let flags = EnvelopeInterpolationFlag::read_be(reader)?;

        let mut kind0 = None;
        if reader.stream_position()? - start < size as u64 {
            kind0 = Some(read_aligned_nullstring(reader)?);
        }

        let mut kind1 = None;
        if reader.stream_position()? - start < size as u64 {
            kind1 = Some(read_aligned_nullstring(reader)?);
        }

        Ok(Gradient {
            name,
            envelope_index,
            flags,
            kind0,
            kind1,
        })
    }
}

#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
#[br(repr=u32)]
#[bw(repr=u32)]
pub enum EnvelopeInterpolationFlag {
    Curve,
    Linear,
    Stepped,
}

#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, BinWrite, Clone, PartialEq)]
pub struct VectorChannel {
    #[bw(align_after=2)]
    pub name: NullString,
    #[bw(map = |mask: &ChannelDataMask| mask.bits())]
    pub kind: ChannelDataMask,
    pub dimensions: u16,
    pub elements: Vec<VectorElement>,
}

impl BinRead for VectorChannel {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> BinResult<Self> {
        let name = read_aligned_nullstring(reader)?;
        let kind = ChannelDataMask::from_bits_retain(u16::read_be(reader)?);
        let dimensions = u16::read_be(reader)?;
        let mut elements = Vec::with_capacity(dimensions as usize);
        for _ in 0..dimensions {
            elements.push(VectorElement::read_be_args(reader, &kind)?);
        }
        Ok(VectorChannel {
            name,
            kind,
            dimensions,
            elements,
        })
    }
}

#[derive(Debug, BinWrite, Clone, PartialEq)]
pub struct VectorElement {
    #[bw(align_after=2)]
    pub name: NullString,
    pub value: ChannelValue,
}

impl BinRead for VectorElement {
    type Args<'a> = &'a ChannelDataMask;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        kind: Self::Args<'_>,
    ) -> BinResult<VectorElement> {
        let name = read_aligned_nullstring(reader)?;
        let value = read_channel_value(reader, kind)?;
        Ok(VectorElement { name, value })
    }
}

///
/// The CHNS sub-chunk represents a string channel containing the channel name and the string 
/// value. Zero or more of these may be present in an ITEM chunk.
///
#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
pub struct StringChannel {
    /// Channel name
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,

    /// String value assigned to the channel
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub value: NullString,
}

#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
pub struct ItemTag {
    pub kind: ID4,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub tag: NullString,
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ChannelDataMask: u16 {
        const Integer   = 0b0001;
        const Float     = 0b0010;
        const String    = 0b0100;
        const Data      = 0b1000;
    }
}

fn read_channel_value<R: Read + Seek>(
    reader: &mut R,
    kind: &ChannelDataMask,
) -> Result<ChannelValue, binrw::Error> {
    if kind.contains(ChannelDataMask::Integer) {
        Ok(ChannelValue::Integer(i32::read_be(reader)?))
    } else if kind.contains(ChannelDataMask::Float) {
        Ok(ChannelValue::Float(f32::read_be(reader)?))
    } else if kind.contains(ChannelDataMask::String) {
        Ok(ChannelValue::String(read_aligned_nullstring(reader)?))
    } else if kind.contains(ChannelDataMask::Data) {
        let size = u16::read_be(reader)?;
        let mut data = vec![0u8; size as usize];
        reader.read_exact(&mut data)?;
        Ok(ChannelValue::Data(data))
    } else {
        Err(binrw::Error::Custom {
            pos: reader.stream_position()?,
            err: Box::new(ParseError::InvalidChannelDataMask),
        })
    }
}

#[derive(Debug, BinWrite, Clone, PartialEq)]
pub struct ScalarChannel {
    #[bw(align_after = 2)]
    pub name: NullString,
    #[bw(map=|mask: &ChannelDataMask| mask.bits())]
    pub kind: ChannelDataMask,
    pub value: ChannelValue,
}

impl BinRead for ScalarChannel {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        _: Self::Args<'_>,
    ) -> BinResult<Self> {
        let name = read_aligned_nullstring(reader)?;
        let kind = ChannelDataMask::from_bits_retain(u16::read_be(reader)?);
        let value = read_channel_value(reader, &kind)?;
        Ok(ScalarChannel { name, kind, value })
    }
}

#[derive(BinRead, BinWrite, Debug, Clone, PartialEq)]
pub struct BlockChannel {
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Channels {
    CHAN(Channel),
    GRAD(Gradient),
    CHNL(ScalarChannel),
    CHNV(VectorChannel),
    CHNS(StringChannel),
    BCHN(BlockChannel),
}

impl Channels {
    fn subchunk_kind(&self) -> ID4 {
        match self {
            Channels::CHAN(_) => ID4::from_str("CHAN").unwrap(),
            Channels::GRAD(_) => ID4::from_str("GRAD").unwrap(),
            Channels::CHNL(_) => ID4::from_str("CHNL").unwrap(),
            Channels::CHNV(_) => ID4::from_str("CHNV").unwrap(),
            Channels::CHNS(_) => ID4::from_str("CHNS").unwrap(),
            Channels::BCHN(_) => ID4::from_str("BCHN").unwrap(),
        }
    }
}

impl BinWrite for Channels {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<()> {
        match self {
            Channels::CHAN(ch) => ch.write_options(writer, _endian, ()),
            Channels::GRAD(ch) => ch.write_options(writer, _endian, ()),
            Channels::CHNL(ch) => ch.write_options(writer, _endian, ()),
            Channels::CHNV(ch) => ch.write_options(writer, _endian, ()),
            Channels::CHNS(ch) => ch.write_options(writer, _endian, ()),
            Channels::BCHN(ch) => ch.write_options(writer, _endian, ()),
        }
    }
}

pub struct DataBlock {
    pub index: u32,
    pub flag: u32,
    pub name: NullString,
    pub data: Vec<u8>,
}

impl BinRead for DataBlock {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()? as u32;
        let index = u32::read_be(reader)?;
        let flag = u32::read_be(reader)?;
        let name = read_aligned_nullstring(reader)?;
        let pos = reader.stream_position()? as u32;
        let mut data: Vec<u8> = vec![0u8; (size - (pos - start)) as usize];
        reader.read_exact(&mut data)?;

        Ok(DataBlock {
            index,
            flag,
            name,
            data,
        })
    }
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
pub struct ChannelLink {
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub graph: NullString,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub from: NullString,
    pub item: u32,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub to: NullString,
    pub from_index: u32,
    pub to_index: u32,
}

#[derive(BinRead, BinWrite, Debug, PartialEq, Eq)]
#[br(repr=u32, big)]
#[bw(repr=u32, big)]
pub enum VectorMode {
    Scalar,
    XY,
    XYZ,
    RGB,
    RGBA,
}

#[derive(BinRead, BinWrite, Debug, PartialEq)]
pub struct TextHint {
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,
    pub value: i32,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct UserChannel {
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub kind: NullString,
    pub mode: VectorMode,
    pub flag: u32,
    pub default_int: i32,
    pub default_float: f32,
    pub num_hints: u16,
    #[br(count=num_hints)]
    pub hints: Vec<TextHint>,
}

#[derive(Debug)]
pub struct Item {
    pub kind: NullString,
    pub name: NullString,
    pub id: u32,

    pub reference: Option<Reference>,

    pub package: Option<Package>,

    pub layer: Option<Layer>,

    pub user_channels: Vec<UserChannel>,
    pub channel_links: Vec<ChannelLink>,

    pub links: Vec<Link>,

    pub channels: Vec<Channels>,

    pub tags: Vec<ItemTag>,

    pub ident: Option<NullString>,
    pub index: Option<u32>,

    pub visible_name: Option<NullString>,

    pub bounds: Option<BoundingBox>,
}

impl ReadEndian for Item {
    const ENDIAN: EndianKind = EndianKind::Endian(Endian::Big);
}

impl BinRead for Item {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let kind = read_aligned_nullstring(reader)?;
        let name = read_aligned_nullstring(reader)?;
        let id = u32::read_be(reader)?;

        let mut reference = None;
        let mut layer = None;
        let mut package = None;
        let mut user_channels = vec![];
        let mut channel_links = vec![];
        let mut links = vec![];
        let mut channels = vec![];
        let mut tags = vec![];
        let mut ident = None;
        let mut index = None;
        let mut visible_name = None;
        let mut bounds = None;

        while reader.stream_position()? - start < size as u64 {
            let header = SubChunkHeader::read_be(reader)?;

            match header.kind.as_str() {
                "XREF" => reference = Some(Reference::read_be(reader)?),
                "LAYR" => layer = Some(Layer::read_be(reader)?),
                "PAKG" => package = Some(Package::read_be(reader)?),
                "UCHN" => user_channels.push(UserChannel::read_be(reader)?),
                "CLNK" => channel_links.push(ChannelLink::read_be(reader)?),
                "LINK" => links.push(Link::read(reader)?),
                "GRAD" => {
                    channels.push(Channels::GRAD(Gradient::read_be_args(reader, header.size)?))
                }
                "CHNL" => channels.push(Channels::CHNL(ScalarChannel::read_be(reader)?)),
                "CHNV" => channels.push(Channels::CHNV(VectorChannel::read_be(reader)?)),
                "CHNS" => channels.push(Channels::CHNS(StringChannel::read_be(reader)?)),
                "CHAN" => channels.push(Channels::CHAN(Channel::read_be(reader)?)),
                "BCHN" => channels.push(Channels::BCHN(BlockChannel::read_be(reader)?)),
                "ITAG" => tags.push(ItemTag::read_be(reader)?),
                "UNIQ" => ident = Some(read_aligned_nullstring(reader)?),
                "UIDX" => index = Some(u32::read_be(reader)?),
                "VNAM" => visible_name = Some(read_aligned_nullstring(reader)?),
                "BBOX" => bounds = Some(BoundingBox::read_be(reader)?),
                _ => {
                    let pos = reader.stream_position()?;
                    eprintln!(
                        "Unknown item subchunk {} at {} size: {}",
                        header.kind,
                        pos - 6,
                        header.size + 6,
                    );
                    reader.seek_relative(header.size as i64)?;
                }
            }
        }

        Ok(Item {
            kind,
            name,
            id,
            reference,
            layer,
            package,
            user_channels,
            channel_links,
            links,
            channels,
            tags,
            ident,
            index,
            visible_name,
            bounds,
        })
    }
}

impl BinWrite for Item {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endiant: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<()> {
        write_aligned_nullstring(writer, &self.kind)?;
        write_aligned_nullstring(writer, &self.name)?;
        self.id.write_be(writer)?;

        if let Some(reference) = &self.reference {
            let mut buf = Cursor::new(Vec::new());
            reference.write_be(&mut buf)?;
            let data = buf.into_inner();
            SubChunkHeader{
                kind: ID4::from_str("XREF").unwrap(),
                size: data.len() as u16,
            }.write_be(writer)?;
            writer.write_all(&data)?;
        }

        if let Some(layer) = &self.layer {
            SubChunkHeader{
                kind: ID4::from_str("LAYR").unwrap(),
                size: 12u16,
            }.write_be(writer)?;
            layer.write_be(writer)?;
        }

        if let Some(package) = &self.package {
            let mut buf = Cursor::new(Vec::new());
            package.write_be(&mut buf)?;
            let data = buf.into_inner();
            SubChunkHeader{
                kind: ID4::from_str("XREF").unwrap(),
                size: data.len() as u16,
            }.write_be(writer)?;
            writer.write_all(&data)?;
        }

        for channel in &self.user_channels {
            let mut buf = Cursor::new(Vec::new());
            channel.write_be(&mut buf)?;
            let data = buf.into_inner();
            SubChunkHeader{
                kind: ID4::from_str("UCHN").unwrap(),
                size: data.len() as u16,
            }.write_be(writer)?;
            writer.write_all(&data)?;
        }

        for channel_link in &self.channel_links {
            let mut buf = Cursor::new(Vec::new());
            channel_link.write_be(&mut buf)?;
            let data = buf.into_inner();
            SubChunkHeader{
                kind: ID4::from_str("CLNK").unwrap(),
                size: data.len() as u16,
            }.write_be(writer)?;
            writer.write_all(&data)?;
        }

        for link in &self.links {
            let mut buf = Cursor::new(Vec::new());
            link.write_be(&mut buf)?;
            let data = buf.into_inner();
            SubChunkHeader{
                kind: ID4::from_str("LINK").unwrap(),
                size: data.len() as u16,
            }.write_be(writer)?;
            writer.write_all(&data)?;
        }

        for channel in &self.channels {
            let mut buf = Cursor::new(Vec::new());
            channel.write_be(&mut buf)?;
            let data = buf.into_inner();
            SubChunkHeader {
                kind: channel.subchunk_kind(),
                size: data.len() as u16,
            }
            .write_be(writer)?;
            writer.write_all(&data)?;
        }

        for tag in &self.tags {
            let mut buf = Cursor::new(Vec::new());
            tag.write_be(&mut buf)?;
            let data = buf.into_inner();
            SubChunkHeader{
                kind: ID4::from_str("ITAG").unwrap(),
                size: data.len() as u16,
            }.write_be(writer)?;
            writer.write_all(&data)?;
        }

        if let Some(ident) = &self.ident {
            let mut buf = Cursor::new(Vec::new());
            write_aligned_nullstring(&mut buf, ident)?;
            let data = buf.into_inner();
            SubChunkHeader{
                kind: ID4::from_str("UNIQ").unwrap(),
                size: data.len() as u16,
            }.write_be(writer)?;
            writer.write_all(&data)?;
        }

        if let Some(index) = &self.index {
            SubChunkHeader{
                kind: ID4::from_str("UIDX").unwrap(),
                size: 4u16,
            }.write_be(writer)?;
            index.write_be(writer)?;
        }

        if let Some(visible_name) = &self.visible_name {
            let mut buf = Cursor::new(Vec::new());
            write_aligned_nullstring(&mut buf, visible_name)?;
            let data = buf.into_inner();
            SubChunkHeader{
                kind: ID4::from_str("VNAM").unwrap(),
                size: data.len() as u16,
            }.write_be(writer)?;
            writer.write_all(&data)?;
        }

        if let Some(bounds) = &self.bounds {
            SubChunkHeader{
                kind: ID4::from_str("BBOX").unwrap(),
                size: 24u16,
            }.write_be(writer)?;
            bounds.write_be(writer)?;
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
    fn reference() {
        let mut reader = Cursor::new([
            0x58, 0x52, 0x45, 0x46, 0x00, 0x2e, 0x00, 0x00, 0x00, 0x7, 0x43, 0x3a, 0x50, 0x72,
            0x6f, 0x6a, 0x65, 0x63, 0x74, 0x2f, 0x50, 0x61, 0x74, 0x68, 0x2f, 0x54, 0x6f, 0x2f,
            0x52, 0x65, 0x66, 0x65, 0x72, 0x65, 0x6e, 0x63, 0x65, 0x2e, 0x6c, 0x78, 0x6f, 0x00,
            0x63, 0x61, 0x6d, 0x65, 0x72, 0x61, 0x30, 0x30, 0x36, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let xref = Reference::read_be(&mut reader).unwrap();

        assert_eq!(xref.index, 7);
        assert_eq!(xref.path, "C:Project/Path/To/Reference.lxo".into());
        assert_eq!(xref.ident, "camera006".into());

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&xref).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn visible_background_layer() {
        let mut reader = Cursor::new([
            0x4c, 0x41, 0x59, 0x52, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x09,
            0xc8, 0xc8, 0xc8, 0xff,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let layer = Layer::read_be(&mut reader).unwrap();

        assert_eq!(layer.index, 3);
        assert_eq!(
            layer.flags,
            LayerVisibilityFlags::Visible | LayerVisibilityFlags::Background
        );
        assert_eq!(layer.color, [200, 200, 200, 255]);

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&layer).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn uistate_package() {
        let mut reader = Cursor::new([
            0x50, 0x41, 0x4b, 0x47, 0x00, 0x0c, 0x75, 0x69, 0x73, 0x74, 0x61, 0x74, 0x65, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let package = Package::read_be(&mut reader).unwrap();

        assert_eq!(package.name, "uistate".into());
        assert_eq!(package.size, 0);
        assert!(package.data.is_empty());

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&package).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn channel_data_mask() {
        let mask = ChannelDataMask::from_bits_retain(2u16);
        assert!(mask.contains(ChannelDataMask::Float));
        assert_eq!(mask, ChannelDataMask::Float);

        // Not sure what this extra bit means here... but this is seen on color
        // related channel data types. Seen on renderer, environment and materials
        let mask = ChannelDataMask::from_bits_retain(34u16);
        assert!(mask.contains(ChannelDataMask::Float));
    }

    #[test]
    fn position_vector_channel() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x4e, 0x56, 0x00, 0x1a, 0x70, 0x6f, 0x73, 0x00, 0x00, 0x22, 0x00, 0x03,
            0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5a, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let vc = VectorChannel::read_be(&mut reader).unwrap();

        assert_eq!(vc.name, "pos".into());
        assert!(vc.kind.contains(ChannelDataMask::Float));
        assert_eq!(vc.dimensions, 3);

        assert_eq!(
            vc.elements[0],
            VectorElement {
                name: "X".into(),
                value: ChannelValue::Float(0.0)
            }
        );
        assert_eq!(
            vc.elements[1],
            VectorElement {
                name: "Y".into(),
                value: ChannelValue::Float(0.0)
            }
        );
        assert_eq!(
            vc.elements[2],
            VectorElement {
                name: "Z".into(),
                value: ChannelValue::Float(0.0)
            }
        );

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&vc).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn ambient_color_vector_channel() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x4e, 0x56, 0x00, 0x20, 0x61, 0x6d, 0x62, 0x43, 0x6f, 0x6c, 0x6f, 0x72,
            0x00, 0x00, 0x00, 0x22, 0x00, 0x03, 0x52, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x47, 0x00,
            0x3f, 0x80, 0x00, 0x00, 0x42, 0x00, 0x3f, 0x80, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let vc = VectorChannel::read_be(&mut reader).unwrap();

        assert_eq!(vc.name, "ambColor".into());
        // kind here is 34u16, so not sure what 0x20 bit represent here but it's only
        // seen in 'color' related CHNV
        assert!(vc.kind.contains(ChannelDataMask::Float));
        assert_eq!(vc.dimensions, 3);

        assert_eq!(
            vc.elements[0],
            VectorElement {
                name: "R".into(),
                value: ChannelValue::Float(1.0)
            }
        );
        assert_eq!(
            vc.elements[1],
            VectorElement {
                name: "G".into(),
                value: ChannelValue::Float(1.0)
            }
        );
        assert_eq!(
            vc.elements[2],
            VectorElement {
                name: "B".into(),
                value: ChannelValue::Float(1.0)
            }
        );

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&vc).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn integer_channel() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x01, 0xe8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = Channel::read_be(&mut reader).unwrap();

        assert_eq!(chan.index, VX::U2(488));
        assert_eq!(chan.kind, 1);
        assert_eq!(chan.value, ChannelValue::Integer(1));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());
    }

    #[test]
    fn integer_channel_undefined_action() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xe0, 0x00, 0x21, 0x00, 0x00, 0x00, 0x01,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = Channel::read_be(&mut reader).unwrap();

        assert_eq!(chan.index, VX::U2(224));
        assert_eq!(chan.kind, 0x21);
        assert_eq!(chan.value, ChannelValue::Integer(1));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());
    }

    #[test]
    fn float_channel() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xbd, 0x00, 0x02, 0x3f, 0x80, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = Channel::read_be(&mut reader).unwrap();

        assert_eq!(chan.index, VX::U2(189));
        assert_eq!(chan.kind, 0x02);
        assert_eq!(chan.value, ChannelValue::Float(1.0));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());
    }

    #[test]
    fn float_channel_undefined_action() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xee, 0x00, 0x22, 0x3f, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = Channel::read_be(&mut reader).unwrap();

        assert_eq!(chan.index, VX::U2(238));
        assert_eq!(chan.kind, 0x22);
        assert_eq!(chan.value, ChannelValue::Float(0.5));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());
    }

    #[test]
    fn channel_with_string_value() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xa4, 0x00, 0x03, 0x6f, 0x66, 0x66, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = Channel::read_be(&mut reader).unwrap();

        assert_eq!(chan.index, VX::U2(164));
        assert_eq!(chan.kind, 0x03);
        assert_eq!(chan.value, ChannelValue::String("off".into()));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&chan).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn channel_with_string_value_and_undefined_action() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x0c, 0x00, 0x47, 0x00, 0x23, 0x64, 0x65, 0x66, 0x61,
            0x75, 0x6c, 0x74, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = Channel::read_be(&mut reader).unwrap();

        assert_eq!(chan.index, VX::U2(71));
        assert_eq!(chan.kind, 0x23);
        assert_eq!(chan.value, ChannelValue::String("default".into()));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&chan).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn gradient_channel() {
        let mut reader = Cursor::new([
            0x47, 0x52, 0x41, 0x44, 0x00, 0x0e, 0x72, 0x61, 0x64, 0x47, 0x72, 0x61, 0x64, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let grad = Gradient::read_be(&mut reader).unwrap();

        assert_eq!(grad.name, "radGrad".into());
        assert_eq!(grad.envelope_index, VX::U2(0));
        assert_eq!(grad.flags, EnvelopeInterpolationFlag::Curve);

        assert_eq!(grad.kind0, None);
        assert_eq!(grad.kind1, None);

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&grad).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn gradient_channel_with_extra_strings() {
        let mut reader = Cursor::new([
            0x47, 0x52, 0x41, 0x44, 0x00, 0x1a, 0x72, 0x61, 0x64, 0x47, 0x72, 0x61, 0x64, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x6c, 0x6f, 0x61, 0x74, 0x00, 0x66, 0x6c,
            0x6f, 0x61, 0x74, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let grad = Gradient::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(grad.name, "radGrad".into());
        assert_eq!(grad.envelope_index, VX::U2(0));
        assert_eq!(grad.flags, EnvelopeInterpolationFlag::Curve);

        assert_eq!(grad.kind0, Some(NullString("float".into())));
        assert_eq!(grad.kind1, Some(NullString("float".into())));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&grad).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn output_pattern_string_channel() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x4e, 0x53, 0x00, 0x2c, 0x6f, 0x75, 0x74, 0x50, 0x61, 0x74,
            0x00, 0x00, 0x2e, 0x5b, 0x3c, 0x70, 0x61, 0x73, 0x73, 0x3e, 0x2e, 0x5d,
            0x5b, 0x3c, 0x6f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x3e, 0x2e, 0x5d, 0x5b,
            0x3c, 0x4c, 0x52, 0x3e, 0x2e, 0x5d, 0x3c, 0x46, 0x46, 0x46, 0x46, 0x3e,
            0x00, 0x00
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let string_channel = StringChannel::read_be(&mut reader).unwrap();

        assert_eq!(string_channel.name, "outPat".into());
        assert_eq!(string_channel.value, ".[<pass>.][<output>.][<LR>.]<FFFF>".into());

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&string_channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn matrix_user_channel() {
        let mut reader = Cursor::new([
            0x55, 0x43, 0x48, 0x4e, 0x00, 0x34, 0x4d, 0x79, 0x4d, 0x61, 0x74, 0x72, 0x69, 0x78,
            0x00, 0x00, 0x6d, 0x61, 0x74, 0x72, 0x69, 0x78, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x23, 0x4d, 0x79, 0x20, 0x4d, 0x61, 0x74, 0x72, 0x69, 0x78, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ]);
        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "UCHN");
        assert_eq!(header.size, 52);

        let user_channel = UserChannel::read_be(&mut reader).unwrap();
        assert_eq!(user_channel.name, "MyMatrix".into());
        assert_eq!(user_channel.kind, "matrix4".into());

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&user_channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn pattern_user_channel() {
        let mut reader = Cursor::new([
            0x55, 0x43, 0x48, 0x4e, 0x00, 0x36, 0x4d, 0x79, 0x50, 0x61, 0x74, 0x74, 0x65, 0x72,
            0x6e, 0x00, 0x2b, 0x70, 0x61, 0x74, 0x74, 0x65, 0x72, 0x6e, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x23, 0x4d, 0x79, 0x20, 0x50, 0x61, 0x74, 0x74, 0x65, 0x72, 0x6e, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "UCHN");
        assert_eq!(header.size, 54);

        let user_channel = UserChannel::read_be(&mut reader).unwrap();
        assert_eq!(user_channel.name, "MyPattern".into());
        assert_eq!(user_channel.kind, "+pattern".into());

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&user_channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn int_range_user_channel() {
        let mut reader = Cursor::new([
            0x55, 0x43, 0x48, 0x4e, 0x00, 0x36, 0x4d, 0x79, 0x4e, 0x75, 0x6d, 0x62, 0x65, 0x72,
            0x73, 0x00, 0x2b, 0x69, 0x6e, 0x74, 0x72, 0x61, 0x6e, 0x67, 0x65, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x23, 0x4d, 0x79, 0x20, 0x4e, 0x75, 0x6d, 0x62, 0x65, 0x72, 0x73, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "UCHN");
        assert_eq!(header.size, 54);

        let user_channel = UserChannel::read_be(&mut reader).unwrap();
        assert_eq!(user_channel.name, "MyNumbers".into());
        assert_eq!(user_channel.kind, "+intrange".into());

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&user_channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn gradient_user_channel() {
        let mut reader = Cursor::new([
            0x55, 0x43, 0x48, 0x4e, 0x00, 0x38, 0x4d, 0x79, 0x47, 0x72, 0x61, 0x64, 0x69, 0x65,
            0x6e, 0x74, 0x00, 0x00, 0x63, 0x6f, 0x6c, 0x6f, 0x72, 0x31, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x23, 0x4d, 0x79, 0x20, 0x47, 0x72, 0x61, 0x64, 0x69, 0x65, 0x6e, 0x74,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "UCHN");
        assert_eq!(header.size, 56);

        let user_channel = UserChannel::read_be(&mut reader).unwrap();
        assert_eq!(user_channel.name, "MyGradient".into());
        assert_eq!(user_channel.kind, "color1".into());

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&user_channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn quaternion_user_channel() {
        let mut reader = Cursor::new([
            0x55, 0x43, 0x48, 0x4e, 0x00, 0x40, 0x4d, 0x79, 0x51, 0x75, 0x61, 0x74, 0x65, 0x72,
            0x6e, 0x69, 0x6f, 0x6e, 0x00, 0x00, 0x71, 0x75, 0x61, 0x74, 0x65, 0x72, 0x6e, 0x69,
            0x6f, 0x6e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x23, 0x4d, 0x79, 0x20, 0x51, 0x75,
            0x61, 0x74, 0x65, 0x72, 0x6e, 0x69, 0x6f, 0x6e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "UCHN");
        assert_eq!(header.size, 64);

        let user_channel = UserChannel::read_be(&mut reader).unwrap();
        assert_eq!(user_channel.name, "MyQuaternion".into());
        assert_eq!(user_channel.kind, "quaternion".into());

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&user_channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn float_user_channel() {
        let mut reader = Cursor::new([
            0x55, 0x43, 0x48, 0x4e, 0x00, 0x38, 0x4d, 0x79, 0x53, 0x69, 0x7a, 0x65, 0x00, 0x00,
            0x66, 0x6c, 0x6f, 0x61, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x00, 0x02, 0x25, 0x6d, 0x69, 0x6e,
            0x00, 0x00, 0x00, 0x00, 0x03, 0xe8, 0x23, 0x4d, 0x79, 0x20, 0x53, 0x69, 0x7a, 0x65,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "UCHN");
        assert_eq!(header.size, 56);

        let user_channel = UserChannel::read_be(&mut reader).unwrap();
        assert_eq!(user_channel.name, "MySize".into());
        assert_eq!(user_channel.kind, "float".into());
        assert_eq!(user_channel.mode, VectorMode::Scalar);
        assert_eq!(user_channel.flag, 0);

        assert_eq!(user_channel.hints.len(), 2);
        assert!(user_channel.hints.contains(&TextHint {
            name: "#My Size".into(),
            value: 0
        }));
        assert!(user_channel.hints.contains(&TextHint {
            name: "%min".into(),
            value: 1000
        }));

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&user_channel).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn test_item() {
        let mut reader = Cursor::new([
            0x6d, 0x65, 0x73, 0x68, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4c, 0x41,
            0x59, 0x52, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xc8, 0xc8,
            0xc8, 0xff, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xe0, 0x00, 0x21, 0x00, 0x00,
            0x00, 0x01, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xe1, 0x00, 0x21, 0x00, 0x00,
            0x00, 0x01, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xe2, 0x00, 0x21, 0x00, 0x00,
            0x00, 0x01, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x0c, 0x01, 0xfa, 0x00, 0x23, 0x64, 0x65,
            0x66, 0x61, 0x75, 0x6c, 0x74, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x0c, 0x01, 0x74,
            0x00, 0x23, 0x64, 0x65, 0x66, 0x61, 0x75, 0x6c, 0x74, 0x00, 0x43, 0x48, 0x41, 0x4e,
            0x00, 0x0c, 0x01, 0x93, 0x00, 0x23, 0x64, 0x65, 0x66, 0x61, 0x75, 0x6c, 0x74, 0x00,
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x0c, 0x01, 0x11, 0x00, 0x23, 0x64, 0x65, 0x66, 0x61,
            0x75, 0x6c, 0x74, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x01, 0xa5, 0x00, 0x02,
            0x3f, 0x80, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x0c, 0x00, 0x94, 0x00, 0x23,
            0x64, 0x65, 0x66, 0x61, 0x75, 0x6c, 0x74, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x0a,
            0x01, 0x0e, 0x00, 0x23, 0x6e, 0x6f, 0x6e, 0x65, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e,
            0x00, 0x0c, 0x00, 0x3e, 0x00, 0x23, 0x64, 0x65, 0x66, 0x61, 0x75, 0x6c, 0x74, 0x00,
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x0c, 0x01, 0x45, 0x00, 0x23, 0x64, 0x65, 0x66, 0x61,
            0x75, 0x6c, 0x74, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0x90, 0x00, 0x22,
            0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xd5, 0x00, 0x21,
            0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x01, 0xa4, 0x00, 0x21,
            0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x0a, 0x00, 0x67, 0x00, 0x23,
            0x77, 0x6f, 0x72, 0x6c, 0x64, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x01, 0x44,
            0x00, 0x22, 0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0x5e,
            0x00, 0x21, 0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x0c, 0x01, 0x53,
            0x00, 0x23, 0x6d, 0x65, 0x74, 0x65, 0x72, 0x73, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e,
            0x00, 0x08, 0x01, 0x55, 0x00, 0x22, 0x3d, 0x4c, 0xcc, 0xcd, 0x47, 0x52, 0x41, 0x44,
            0x00, 0x0e, 0x72, 0x61, 0x64, 0x47, 0x72, 0x61, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x47, 0x52, 0x41, 0x44, 0x00, 0x0e, 0x72, 0x6f, 0x74, 0x47, 0x72, 0x61,
            0x64, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x01, 0x47, 0x00, 0x21, 0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x01, 0xa3, 0x00, 0x21, 0x00, 0x00, 0x00, 0x08, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x01, 0xbf, 0x00, 0x22, 0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x00, 0x9a, 0x00, 0x22, 0x3f, 0x80, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x01, 0x42, 0x00, 0x21, 0x00, 0x00, 0x00, 0x01, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x00, 0x65, 0x00, 0x21, 0x00, 0x00, 0x00, 0x00, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x00, 0x64, 0x00, 0x21, 0x00, 0x00, 0x00, 0x01, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x00, 0x66, 0x00, 0x21, 0x00, 0x00, 0x00, 0x01, 0x55, 0x4e, 0x49, 0x51, 0x00, 0x08,
            0x6d, 0x65, 0x73, 0x68, 0x30, 0x30, 0x32, 0x00, 0x55, 0x49, 0x44, 0x58, 0x00, 0x04,
            0x00, 0x00, 0x00, 0x01, 0x56, 0x4e, 0x41, 0x4d, 0x00, 0x06, 0x4d, 0x65, 0x73, 0x68,
            0x00, 0x00, 0x42, 0x42, 0x4f, 0x58, 0x00, 0x18, 0xbf, 0x00, 0x00, 0x00, 0xbf, 0x00,
            0x00, 0x00, 0xbf, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00,
            0x3f, 0x00, 0x00, 0x00,
        ]);

        let item = Item::read_be_args(&mut reader, 564).unwrap();

        assert_eq!(item.kind, "mesh".into());
        assert!(item.name.is_empty());
        assert_eq!(item.id, 0);

        assert!(item.reference.is_none());
        assert_eq!(
            item.layer,
            Some(Layer {
                index: 0,
                flags: LayerVisibilityFlags::Visible | LayerVisibilityFlags::Foreground,
                color: [200, 200, 200, 255]
            })
        );
        assert!(item.links.is_empty());
        assert_eq!(item.channels.len(), 30);
        assert!(item.tags.is_empty());
        assert_eq!(item.ident, Some("mesh002".into()));
        assert_eq!(item.index, Some(1));
        assert_eq!(item.visible_name, Some("Mesh".into()));
        assert_eq!(
            item.bounds,
            Some(BoundingBox {
                min: [-0.5, -0.5, -0.5],
                max: [0.5, 0.5, 0.5]
            })
        );

        // assert we read all data
        assert!(reader.stream_position().unwrap() == reader.get_ref().len() as u64);

        let mut writer = Cursor::new(vec![]);
        item.write_be(&mut writer).unwrap();
        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn channel_link() {
        let mut reader = Cursor::new([
            0x43, 0x4c, 0x4e, 0x4b, 0x00, 0x24, 0x63, 0x68, 0x61, 0x6e, 0x4c, 0x69,
            0x6e, 0x6b, 0x73, 0x00, 0x6f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x27, 0x73, 0x69, 0x7a, 0x65, 0x59, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        let link = ChannelLink::read_be(&mut reader).unwrap();

        assert_eq!(link.graph, "chanLinks".into());
        assert_eq!(link.from, "output".into());
        assert_eq!(link.item, 39);
        assert_eq!(link.to, "sizeY".into());
        assert_eq!(link.from_index, 1);
        assert_eq!(link.to_index, 0);

        assert_eq!(reader.stream_position().unwrap(), (header.size + 6).into());

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&link).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn data_block() {
        // These are related to user channels, and block channels
        let mut reader = Cursor::new([
            0x44, 0x41, 0x54, 0x41, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x2b, 0x70, 0x61, 0x74, 0x74, 0x65, 0x72, 0x6e, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x44, 0x41, 0x54, 0x41, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x2b, 0x69, 0x6e, 0x74, 0x72, 0x61, 0x6e, 0x67, 0x65, 0x00, 0x33, 0x2c,
            0x37, 0x2c, 0x32, 0x2c, 0x31, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let data = DataBlock::read_be_args(&mut reader, header.size).unwrap();
        assert_eq!(data.index, 0);
        assert_eq!(data.flag, 0);
        assert_eq!(data.name, "+pattern".into());

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let data = DataBlock::read_be_args(&mut reader, header.size).unwrap();
        assert_eq!(data.index, 1);
        assert_eq!(data.flag, 0);
        assert_eq!(data.name, "+intrange".into());

        assert!(reader.stream_position().unwrap() == reader.get_ref().len() as u64);
    }
}
