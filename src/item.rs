use crate::primitives::{ChannelValue, ID4, SubChunkHeader, VX};
use crate::utils::read_aligned_nullstring;
use binrw::meta::{EndianKind, ReadEndian};
use binrw::{
    BinRead, BinResult, Endian, NullString,
    io::{Read, Seek},
};

#[derive(BinRead, Debug, Clone, PartialEq)]
#[br(big)]
pub struct Layer {
    pub index: u32,
    pub flags: u32,
    pub color: [u8; 4],
}

// not to be confused with the Chunk XREF, this is a subchunk in item
#[derive(BinRead, Debug)]
pub struct Reference {
    pub index: u32,
    #[br(align_after = 2)]
    pub path: NullString,
    #[br(align_after = 2)]
    pub ident: NullString,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
#[br(big)]
pub struct Package {
    #[br(align_after = 2)]
    name: NullString,
    size: u32,
    #[br(count = size)]
    data: Vec<u8>,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
#[br(big)]
pub struct Channel {
    index: VX,
    kind: u16,
    #[br(args(kind))]
    value: ChannelValue,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
#[br(big)]
pub struct Link {
    #[br(align_after = 2)]
    pub name: NullString,
    pub id: u32,
    pub index: u32,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
#[br(big)]
pub struct Gradient {
    #[br(align_after = 2)]
    pub name: NullString,
    pub envelope_index: VX,
    pub flags: u32,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorChannel {
    pub name: NullString,
    pub kind: u16,
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
        let kind = u16::read_be(reader)?;
        let dimensions = u16::read_be(reader)?;
        let mut elements = Vec::with_capacity(dimensions as usize);
        for _ in 0..dimensions {
            elements.push(VectorElement::read_be_args(reader, kind)?);
        }
        Ok(VectorChannel {
            name,
            kind,
            dimensions,
            elements,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorElement {
    pub name: NullString,
    pub value: ChannelValue,
}

impl BinRead for VectorElement {
    type Args<'a> = u16;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        flag: Self::Args<'_>,
    ) -> BinResult<VectorElement> {
        let name = read_aligned_nullstring(reader)?;
        let value = ChannelValue::read_be_args(reader, (flag,))?;
        Ok(VectorElement { name, value })
    }
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct StringChannel {
    #[br(align_after = 2)]
    pub name: NullString,
    #[br(align_after = 2)]
    pub value: NullString,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct ItemTag {
    pub kind: ID4,
    #[br(align_after = 2)]
    pub tag: NullString,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
#[br(big)]
pub struct ScalarChannel {
    #[br(align_after = 2)]
    pub name: NullString,
    pub kind: u16,
    #[br(args(kind))]
    pub value: ChannelValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Channels {
    CHAN(Channel),
    GRAD(Gradient),
    CHNL(ScalarChannel),
    CHNV(VectorChannel),
    CHNS(StringChannel),
}

#[derive(Debug)]
pub struct Item {
    pub kind: NullString,
    pub name: NullString,
    pub id: u32,

    pub reference: Option<Reference>,

    pub layer: Option<Layer>,

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
                "LINK" => links.push(Link::read(reader)?),
                "GRAD" => channels.push(Channels::GRAD(Gradient::read_be(reader)?)),
                "CHNL" => channels.push(Channels::CHNL(ScalarChannel::read_be(reader)?)),
                "CHNV" => channels.push(Channels::CHNV(VectorChannel::read_be(reader)?)),
                "CHNS" => channels.push(Channels::CHNS(StringChannel::read_be(reader)?)),
                "CHAN" => channels.push(Channels::CHAN(Channel::read_be(reader)?)),
                "ITAG" => tags.push(ItemTag::read_be(reader)?),
                "UNIQ" => ident = Some(read_aligned_nullstring(reader)?),
                "UIDX" => index = Some(u32::read_be(reader)?),
                "VNAM" => visible_name = Some(read_aligned_nullstring(reader)?),
                "BBOX" => bounds = Some(BoundingBox::read_be(reader)?),
                _ => {
                    let pos = reader.stream_position()?;
                    eprintln!(
                        "Unknown item subchunk {} at {} size: {}",
                        header.kind, pos, header.size,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_vector_channel() {
        // Channel vector taken som the Renderer
        let mut reader = Cursor::new([
            0x43, 0x48, 0x4e, 0x56, 0x00, 0x20, // header bytes
            0x61, 0x6d, 0x62, 0x43, 0x6f, 0x6c, 0x6f, 0x72, 0x00, 0x00, // ambColor\0\0
            0x00, 0x22, 0x00, 0x03, // kind is 0x0022 and dimension 0x0003
            0x52, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x47, 0x00, 0x3f, 0x80, 0x00, 0x00, 0x42, 0x00,
            0x3f, 0x80, 0x00, 0x00,
        ]);

        let _ = SubChunkHeader::read_be(&mut reader).unwrap();
        let vc = VectorChannel::read_be(&mut reader).unwrap();

        assert_eq!(vc.name, "ambColor".into());
        assert_eq!(vc.kind, 0x0022); // todo: Get the bitflag enum for this...
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
    }

    #[test]
    fn test_channel() {
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, // CHAN, 8 bytes in size
            0x00, 0xe0, // 0x00e0 - index into Channel Names, inheritPos in this case
            0x00, 0x21, // flag
            0x00, 0x00, 0x00, 0x01, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xe1, 0x00, 0x21,
            0x00, 0x00, 0x00, 0x01, 0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, 0x00, 0xe2, 0x00, 0x21,
            0x00, 0x00, 0x00, 0x01,
        ]);

        let _ = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = Channel::read_be(&mut reader).unwrap();

        assert_eq!(chan.index, VX::U2(224));
        assert_eq!(chan.value, ChannelValue::Integer(1));
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
        assert_eq!(item.layer, Some(Layer{index: 0, flags: 5, color: [200, 200, 200, 255]}));
        assert!(item.links.is_empty());
        assert_eq!(item.channels.len(), 30);
        assert!(item.tags.is_empty());
        assert_eq!(item.ident, Some("mesh002".into()));
        assert_eq!(item.index, Some(1));
        assert_eq!(item.visible_name, Some("Mesh".into()));
        assert_eq!(item.bounds, Some(BoundingBox{min: [-0.5, -0.5, -0.5], max: [0.5, 0.5, 0.5]}));

        // assert we read all data
        assert!(reader.stream_position().unwrap() == reader.get_ref().len() as u64);
    }
}
