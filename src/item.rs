use binrw::{BinRead, Endian, BinResult, io::{Read, Seek}};
use binrw::meta::{ReadEndian, EndianKind};
use crate::primitives::{S0, U2, U4, VX};
use crate::ID4;

#[derive(BinRead, Debug)]
pub struct SubChunkHeader {
    pub kind: ID4,
    pub size: U2,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Layer {
    pub index: U4,
    pub flags: U4,
    pub color: [u8; 4],
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Package {
    name: S0,
    size: U4,
    #[br(count = size)]
    data: Vec<u8>
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Channel {
    index: VX,
    kind: U2,
    #[br(args(kind.0))]
    value: ChannelValue
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Link {
    pub name: S0,
    pub id: U4,
    pub index: U4,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Gradient {
    pub name: S0,
    pub envelope_index: VX,
    pub flags: U4,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct UniqueIdentifier {
    pub identifier: S0,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct UniqueItemIndex {
    pub index: U4,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct VectorChannel {
    pub name: S0,
    pub kind: U2,
    pub dimensions: U2,
    #[br(count = dimensions, args{inner: kind})]
    pub elements: Vec<VectorElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorElement {
    pub name: S0,
    pub value: ChannelValue,
}

impl BinRead for VectorElement {
    type Args<'a> = U2;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        kind: Self::Args<'_>,
    ) -> BinResult<VectorElement> {
        let name = S0::read_be(reader)?;
        match kind.0 & !0x20 {
            0b0000_0001 => {
                let i = i32::read_be(reader)?;
                Ok(VectorElement{name: name, value: ChannelValue::Integer(i)})
            },
            0b0000_0010 => {
                let f = f32::read_be(reader)?;
                Ok(VectorElement{name: name, value: ChannelValue::Float(f)})
            },
            0b0000_0100 => {
                let s = S0::read_be(reader)?;
                Ok(VectorElement{name: name, value: ChannelValue::String(s)})
            },
            _ => {
                let pos = reader.stream_position()?;
                Err(binrw::error::Error::Custom{
                    pos, 
                    err: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("No support for Channel Vector type: {}", kind.0)
                    )),
                })}  // todo: find a lxo where channel has array type, or fuzz modo
        }
    }
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct StringChannel {
    pub name: S0,
    pub value: S0,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct ItemTag {
    pub kind: ID4,
    pub tag: S0,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct VisibleName {
    pub name: S0,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelValue {
    Integer(i32),
    Float(f32),
    String(S0),
}

impl ReadEndian for ChannelValue {
    const ENDIAN: EndianKind = EndianKind::Endian(Endian::Big);
}

impl BinRead for ChannelValue {
    type Args<'a> = (u16,);

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        flag: Self::Args<'_>,
    ) -> BinResult<Self> {
        match flag.0 & !0x20 {
            0x1 | 0x11 => Ok(ChannelValue::Integer(i32::read_options(reader, endian, ())?)),
            0x2 | 0x12 => Ok(ChannelValue::Float(f32::read_options(reader, endian, ())?)),
            0x3 | 0x13 => {
                let s = S0::read_options(reader, endian, ())?;
                Ok(ChannelValue::String(s))
            }
            _ => {
                let pos = reader.stream_position()?;
                panic!("Invalid ItemFlag {} at: {}", flag.0, pos)
            },
        }
    }
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct ScalarChannel {
    pub name: S0,
    pub kind: U2,
    pub value: ChannelValue,
}


#[derive(Debug, Clone, PartialEq)]
pub enum SubChunks {
    LAYR(Layer),
    PAKG(Package),
    CHAN(Channel),
    LINK(Link),
    GRAD(Gradient),
    UNIQ(UniqueIdentifier),
    UIDX(UniqueItemIndex),
    BBOX(BoundingBox),
    CHNL(ScalarChannel),
    CHNV(VectorChannel),
    CHNS(StringChannel),
    ITAG(ItemTag),
    VNAM(VisibleName),
    Unknown { kind: ID4, size: U2 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub kind: S0,
    pub name: S0,
    pub id: U4,
    pub subchunks: Vec<SubChunks>,
}

impl ReadEndian for Item {
    const ENDIAN: EndianKind = EndianKind::Endian(Endian::Big);
}

impl BinRead for Item {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let kind = S0::read_options(reader, endian, ())?;
        let name = S0::read_options(reader, endian, ())?;
        let id = U4::read_options(reader, endian, ())?;

        let mut subchunks = Vec::new();

        while reader.stream_position()? - start < size as u64 {
            let header = SubChunkHeader::read_be(reader)?;

            let subchunk = match header.kind.as_str() {
                "LAYR" => {
                    let chunk = Layer::read_options(reader, endian, ())?;
                    SubChunks::LAYR(chunk)
                }
                "LINK" => {
                    let chunk = Link::read_options(reader, endian, ())?;
                    SubChunks::LINK(chunk)
                }
                "GRAD" => {
                    let chunk = Gradient::read_options(reader, endian, ())?;
                    SubChunks::GRAD(chunk)
                }
                "UNIQ" => {
                    let chunk = UniqueIdentifier::read_options(reader, endian, ())?;
                    SubChunks::UNIQ(chunk)
                }
                "UIDX" => {
                    let chunk = UniqueItemIndex::read_options(reader, endian, ())?;
                    SubChunks::UIDX(chunk)
                }
                "BBOX" => {
                    let chunk = BoundingBox::read_options(reader, endian, ())?;
                    SubChunks::BBOX(chunk)
                }
                "CHNL" => {
                    let chunk = ScalarChannel::read_options(reader, endian, ())?;
                    SubChunks::CHNL(chunk)
                }
                "CHNV" => {
                    let chunk = VectorChannel::read_options(reader, endian, ())?;
                    SubChunks::CHNV(chunk)
                }
                "CHNS" => {
                    let chunk = StringChannel::read_options(reader, endian, ())?;
                    SubChunks::CHNS(chunk)
                }
                "ITAG" => {
                    let chunk = ItemTag::read_options(reader, endian, ())?;
                    SubChunks::ITAG(chunk)
                }
                "VNAM" => {
                    let chunk = VisibleName::read_options(reader, endian, ())?;
                    SubChunks::VNAM(chunk)
                }
                "CHAN" => {
                    let chunk = Channel::read_options(reader, endian, ())?;
                    SubChunks::CHAN(chunk)
                }
                _ => {
                    reader.seek_relative(header.size.0 as i64)?;
                    SubChunks::Unknown { kind: header.kind, size: header.size }
                }
            };
            
            subchunks.push(subchunk);
        }

        Ok(Item { kind, name, id, subchunks })
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
            0x43, 0x48, 0x4e, 0x56, 0x00, 0x20,  // header bytes
            0x61, 0x6d, 0x62, 0x43, 0x6f, 0x6c, 0x6f, 0x72, 0x00, 0x00, // ambColor\0\0
            0x00, 0x22, 0x00, 0x03,  // kind is 0x0022 and dimension 0x0003
            0x52, 0x00,
            0x3f, 0x80, 0x00, 0x00,
            0x47, 0x00,
            0x3f, 0x80, 0x00, 0x00,
            0x42, 0x00,
            0x3f, 0x80, 0x00, 0x00
        ]);

        let _ = SubChunkHeader::read_be(&mut reader).unwrap();
        let vc = VectorChannel::read_be(&mut reader).unwrap();

        assert_eq!(vc.name, "ambColor".into());
        assert_eq!(vc.kind.0, 0x0022);  // todo: Get the bitflag enum for this...
        assert_eq!(vc.dimensions.0, 3);

        assert_eq!(vc.elements[0], VectorElement{name: "R".into(), value: ChannelValue::Float(1.0)});
        assert_eq!(vc.elements[1], VectorElement{name: "G".into(), value: ChannelValue::Float(1.0)});
        assert_eq!(vc.elements[2], VectorElement{name: "B".into(), value: ChannelValue::Float(1.0)});

    }

    #[test]
    fn test_channel() {
        // 0000177a: c8ff 4348 414e 0008 00e0 0021 0000 0001  ..CHAN.....!....
        // 0000178a: 4348 414e 0008 00e1 0021 0000 0001 4348  CHAN.....!....CH
        // 0000179a: 414e 0008 00e2 0021 0000 0001 4348 414e  AN.....!....CHAN
        // 000017aa: 000c
        let mut reader = Cursor::new([
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08, // CHAN, 8 bytes in size
            0x00, 0xe0, // 0x00e0 - index into Channel Names, inheritPos in this case 
            0x00, 0x21, // flag
            0x00, 0x00, 0x00, 0x01,
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x00, 0xe1,
            0x00, 0x21,
            0x00, 0x00, 0x00, 0x01,
            0x43, 0x48, 0x41, 0x4e, 0x00, 0x08,
            0x00, 0xe2,
            0x00, 0x21,
            0x00, 0x00, 0x00, 0x01, 
        ]);

        let _ = SubChunkHeader::read_be(&mut reader).unwrap();
        let chan = Channel::read_be(&mut reader).unwrap();

        assert_eq!(chan.index, VX::U2(224));
        assert_eq!(chan.value, ChannelValue::Integer(1));
    }
}
