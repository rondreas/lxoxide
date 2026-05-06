use binrw::{BinRead, BinResult, NullString};
use std::io::{Read, Seek};
use crate::item::SubChunkHeader;

// todo: create a scene with multiple references to see if each ref get's it's own IASS
// or if each reference is a XREF subchunk for IASS
#[derive(Debug)]
pub struct IncludeAsSubscene {
    pub reference: SubsceneReference
}

#[derive(BinRead, Debug)]
pub struct SubsceneReference {
    #[br(align_after = 2)]
    pub name: NullString,
    #[br(align_after = 2)]
    pub path: NullString,
}

#[derive(Debug)]
pub struct Subscene {
    pub path: NullString,
    pub name: NullString,
    pub item_references: Vec<ItemReference>,
}

#[derive(BinRead, Debug)]
pub struct ItemReference {
    #[br(align_after = 2)]
    pub ident: NullString,
    #[br(align_after = 2)]
    pub name: NullString,
    #[br(align_after = 2)]
    pub kind: NullString,
}

// todo: XREF chunk, no idea what the content means... IDEL & XMAN
#[derive(BinRead, Debug)]
#[br(import(count: u32))]
pub struct Reference(#[br(count = count)] Vec<u8>);


#[derive(Debug)]
pub struct ItemTags {
    pub tags: Vec<NullString>,
}

impl BinRead for ItemTags {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let mut buf: Vec<u8> = vec![0u8; size as usize];
        reader.read_exact(&mut buf)?;

        let tags: Vec<NullString> = buf
            .split(|&c| c == 0u8)
            .filter(|s| !s.is_empty())
            .map(|s| NullString(s.to_vec()))
            .collect();

        Ok(ItemTags { tags })
    }
}

#[derive(Debug)]
pub struct ChannelNames {
    pub count: u32,
    pub names: Vec<NullString>,
}

impl BinRead for ChannelNames {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let count = u32::read_be(reader)?;
        let mut buf: Vec<u8> = vec![0u8; (size - 4) as usize];
        reader.read_exact(&mut buf)?;

        let names: Vec<NullString> = buf
            .split(|&c| c == 0u8)
            .filter(|s| !s.is_empty())
            .map(|s| NullString(s.to_vec()))
            .collect();

        Ok(ChannelNames { count, names })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkHeader;
    use std::io::Cursor;

    #[test]
    fn test_item_tags() {
        let mut reader = Cursor::new([
            0x54, 0x41, 0x47, 0x53, 0x00, 0x00, 0x00, 0x10, 0x44, 0x65, 0x66, 0x61, 0x75, 0x6c,
            0x74, 0x00, 0x44, 0x65, 0x66, 0x61, 0x75, 0x6c, 0x74, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let itags = ItemTags::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(
            itags.tags,
            vec![NullString("Default".into()), NullString("Default".into())]
        );
    }

    #[test]
    fn item_reference() {
        let mut reader = Cursor::new([
            0x49, 0x52, 0x45, 0x46, 0x00, 0x32, 0x72, 0x65, 0x6e, 0x64, 0x65, 0x72,
            0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x30, 0x32, 0x35, 0x00, 0x46, 0x69,
            0x6e, 0x61, 0x6c, 0x20, 0x43, 0x6f, 0x6c, 0x6f, 0x72, 0x20, 0x4f, 0x75,
            0x74, 0x70, 0x75, 0x74, 0x00, 0x00, 0x72, 0x65, 0x6e, 0x64, 0x65, 0x72,
            0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x00
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "IREF");
        assert_eq!(header.size, 50);

        let iref = ItemReference::read_be(&mut reader).unwrap();

        assert_eq!(iref.ident, "renderOutput025".into());
        assert_eq!(iref.name, "Final Color Output".into());
        assert_eq!(iref.kind, "renderOutput".into());
    }
}
