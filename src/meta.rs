use binrw::{BinRead, BinResult, NullString};
use std::io::{Read, Seek};

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
}
