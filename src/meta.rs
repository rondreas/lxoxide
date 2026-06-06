use crate::primitives::SubChunkHeader;
use crate::utils::read_aligned_nullstring;
use binrw::{BinRead, BinResult, BinWrite, NullString};
use std::fmt;
use std::io::{Read, Seek};

#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub application: NullString,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{} {}", self.major, self.minor, self.application)
    }
}

#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct ApplicationVersion {
    pub base: u32,
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub application: NullString,
}

impl fmt::Display for ApplicationVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{} - {} {}",
            self.base, self.major, self.minor, self.build, self.application
        )
    }
}

#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(big, repr = u32)]
#[bw(big, repr = u32)]
pub enum Encoding {
    Default = 0,
    Ansi = 1,
    Utf8 = 2,
    ShiftJis = 3,
    EucJp = 4,
    EucKr = 5,
    Gb2312 = 6,
    Big5 = 7,
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Encoding::Default => "default",
            Encoding::Ansi => "ANSI",
            Encoding::Utf8 => "UTF-8",
            Encoding::ShiftJis => "Shift JIS",
            Encoding::EucJp => "EUC-JP",
            Encoding::EucKr => "EUC-KR",
            Encoding::Gb2312 => "GB2312",
            Encoding::Big5 => "Big5",
        };
        write!(f, "{}", s)
    }
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct Description {
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub kind: NullString,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub description: NullString,
}

/// Preview image for the scene, can be stored either from 3D View, or latest render in Modo on the
/// scene item.
#[derive(BinRead, Debug)]
#[br(import(size: u32), assert(size > (12 + 68)))]
pub struct Preview {
    pub width: u16,
    pub height: u16,
    /// Image pixel format
    pub kind: u32,
    /// Flags, 1 = PNG, Not tested, but think 0 means it's raw byte image, see LWO spec
    pub flags: u32,
    #[br(count = size - 12)]
    pub data: Vec<u8>,
}

// Clips are apparently IASS
#[derive(Debug, BinRead, PartialEq)]
pub struct Flat {
    #[br(align_after = 2)]
    pub name: NullString,
    #[br(align_after = 2)]
    pub source: NullString,
    #[br(align_after = 2)]
    pub kind: NullString,
    #[br(align_after = 2)]
    pub subkind: NullString,
    #[br(align_after = 2)]
    pub path: NullString,
}

#[derive(Debug)]
pub struct IncludeAsSubscene {
    pub references: Vec<SubsceneReference>,
    pub clips: Vec<Flat>,
}

impl BinRead for IncludeAsSubscene {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let mut references = vec![];
        let mut clips = vec![];

        while reader.stream_position()? - start < size as u64 {
            let header = SubChunkHeader::read_be(reader)?;
            match header.kind.as_str() {
                "XREF" => references.push(SubsceneReference::read_be(reader)?),
                "FLAT" => clips.push(Flat::read_be(reader)?),
                _ => {
                    let pos = reader.stream_position()?;
                    reader.seek_relative(header.size as i64)?;
                    eprintln!(
                        "Unknown IASS subchunk {} at {} size {}",
                        header.kind.as_str(),
                        pos - 6,
                        header.size
                    );
                }
            }
        }

        Ok(IncludeAsSubscene { references, clips })
    }
}

#[derive(BinRead, Debug, PartialEq)]
pub struct SubsceneReference {
    #[br(align_after = 2)]
    pub name: NullString,
    #[br(align_after = 2)]
    pub path: NullString,
}

#[derive(BinWrite, Debug)]
#[bw(big)]
pub struct Subscene {
    #[bw(align_after = 2)]
    pub path: NullString,
    #[bw(align_after = 2)]
    pub name: NullString,
    pub item_references: Vec<ItemReference>,
}

impl BinRead for Subscene {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let path = read_aligned_nullstring(reader)?;
        let name = read_aligned_nullstring(reader)?;
        let mut item_references = vec![];
        while reader.stream_position()? - start < size as u64 {
            let header = SubChunkHeader::read_be(reader)?;
            if header.kind == "IREF" {
                item_references.push(ItemReference::read_be(reader)?);
            } else {
                eprintln!("Unknown subchunk of SUBS {}", header.kind);
                reader.seek_relative(header.size as i64)?;
            }
        }
        Ok(Subscene {
            path,
            name,
            item_references,
        })
    }
}

#[derive(BinRead, BinWrite, Debug)]
pub struct ItemReference {
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub ident: NullString,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub name: NullString,
    #[br(align_after = 2)]
    #[bw(align_after = 2)]
    pub kind: NullString,
}

#[derive(BinRead, Debug)]
#[br(import(size: u32))]
pub struct Reference {
    pub subscene: u32,
    #[br(align_after = 2)]
    pub subscene_name: NullString,
    pub item_deletion: ItemDeletion,
    pub reference_manager: ReferenceManager,
}

// Subchunk of XREF
#[derive(BinRead, Debug)]
#[br(big, magic = b"IDEL")]
pub struct ItemDeletion {
    _size: u16,
    #[br(count = _size / 4)]
    pub items: Vec<u32>,
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"XMAN")]
pub struct ReferenceManager {
    _size: u16,
    pub mode: u32,
}

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

#[derive(BinRead, Debug)]
#[br(big)]
pub struct Parent {
    #[br(align_after = 2)]
    pub name: NullString,
    pub num: u32, // todo: better name for field, is it a ref? index into another list of chunks?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkHeader;
    use binrw::BinWriterExt;
    use std::io::Cursor;

    #[test]
    fn version() {
        let mut reader = Cursor::new([
            0x56, 0x52, 0x53, 0x4e, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00,
            0x00, 0x00, 0x6e, 0x65, 0x78, 0x75, 0x73, 0x20, 0x32, 0x30, 0x30, 0x30, 0x20, 0x62,
            0x79, 0x20, 0x54, 0x68, 0x65, 0x20, 0x46, 0x6f, 0x75, 0x6e, 0x64, 0x72, 0x79, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "VRSN");

        let version = Version::read_be(&mut reader).unwrap();
        assert_eq!(
            version,
            Version {
                major: 16,
                minor: 0,
                application: "nexus 2000 by The Foundry".into(),
            }
        );

        assert_eq!(reader.stream_position().unwrap(), 42);

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&version).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn application_version() {
        let mut reader = Cursor::new([
            0x41, 0x50, 0x50, 0x56, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x07, 0xd0, 0x00, 0x00,
            0x07, 0xd0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x17, 0xc6, 0x4d, 0x6f, 0x64, 0x6f,
            0x20, 0x31, 0x36, 0x2e, 0x30, 0x76, 0x31, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "APPV");

        let application_version = ApplicationVersion::read_be(&mut reader).unwrap();
        assert_eq!(
            application_version,
            ApplicationVersion {
                base: 2000,
                major: 2000,
                minor: 0,
                build: 661446,
                application: "Modo 16.0v1".into(),
            }
        );

        assert_eq!(reader.stream_position().unwrap(), 36);

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&application_version).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn encoding_utf8() {
        let mut reader = Cursor::new([
            0x45, 0x4e, 0x43, 0x4f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02,
        ]);
        let header = ChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "ENCO");
        assert_eq!(header.size, 4);

        let encoding = Encoding::read_be(&mut reader).unwrap();
        assert_eq!(encoding, Encoding::Utf8);

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&encoding).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn invalid_encoding() {
        let mut reader = Cursor::new([
            0x45, 0x4e, 0x43, 0x4f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x0c, 0x02,
        ]);
        let header = ChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "ENCO");
        assert_eq!(header.size, 4);

        let result = Encoding::read_be(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn description() {
        let mut reader = Cursor::new([
            0x44, 0x45, 0x53, 0x43, 0x00, 0x00, 0x00, 0x0a, 0x6c, 0x6f, 0x63, 0x61, 0x74, 0x6f,
            0x72, 0x00, 0x00, 0x00,
        ]);
        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let desc = Description::read_be(&mut reader).unwrap();

        assert_eq!(desc.kind, "locator".into());
        assert!(desc.description.is_empty());

        assert_eq!(reader.stream_position().unwrap(), 18);

        let mut writer = Cursor::new(vec![]);
        writer.write_be(&header).unwrap();
        writer.write_be(&desc).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }

    #[test]
    fn empty_include_as_subscene() {
        let mut reader = Cursor::new([0x49, 0x41, 0x53, 0x53, 0x00, 0x00, 0x00, 0x00]);
        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let include_as_subscene =
            IncludeAsSubscene::read_be_args(&mut reader, header.size).unwrap();
        assert!(include_as_subscene.references.is_empty());
    }

    #[test]
    fn include_as_subscene_with_one_reference() {
        let mut reader = Cursor::new([
            0x49, 0x41, 0x53, 0x53, 0x00, 0x00, 0x00, 0x30, 0x58, 0x52, 0x45, 0x46, 0x00, 0x2a,
            0x6d, 0x79, 0x5f, 0x73, 0x63, 0x65, 0x6e, 0x65, 0x00, 0x00, 0x44, 0x3a, 0x5c, 0x70,
            0x72, 0x6f, 0x6a, 0x65, 0x63, 0x74, 0x5c, 0x73, 0x63, 0x65, 0x6e, 0x65, 0x73, 0x5c,
            0x6d, 0x79, 0x5f, 0x73, 0x63, 0x65, 0x6e, 0x65, 0x2e, 0x6c, 0x78, 0x6f, 0x00, 0x00,
        ]);
        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let include_as_subscene =
            IncludeAsSubscene::read_be_args(&mut reader, header.size).unwrap();
        assert_eq!(include_as_subscene.references.len(), 1);
        assert_eq!(
            include_as_subscene.references[0],
            SubsceneReference {
                name: "my_scene".into(),
                path: "D:\\project\\scenes\\my_scene.lxo".into()
            }
        );
    }

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
            0x49, 0x52, 0x45, 0x46, 0x00, 0x32, 0x72, 0x65, 0x6e, 0x64, 0x65, 0x72, 0x4f, 0x75,
            0x74, 0x70, 0x75, 0x74, 0x30, 0x32, 0x35, 0x00, 0x46, 0x69, 0x6e, 0x61, 0x6c, 0x20,
            0x43, 0x6f, 0x6c, 0x6f, 0x72, 0x20, 0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x00,
            0x72, 0x65, 0x6e, 0x64, 0x65, 0x72, 0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x00,
        ]);

        let header = SubChunkHeader::read_be(&mut reader).unwrap();
        assert_eq!(header.kind, "IREF");
        assert_eq!(header.size, 50);

        let iref = ItemReference::read_be(&mut reader).unwrap();

        assert_eq!(iref.ident, "renderOutput025".into());
        assert_eq!(iref.name, "Final Color Output".into());
        assert_eq!(iref.kind, "renderOutput".into());
    }

    #[test]
    fn parent() {
        let mut reader = Cursor::new([
            0x50, 0x52, 0x4e, 0x54, 0x00, 0x00, 0x00, 0x0c, 0x28, 0x6e, 0x6f, 0x6e, 0x65, 0x29,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        let _ = ChunkHeader::read_be(&mut reader).unwrap();
        let parent = Parent::read_be(&mut reader).unwrap();

        assert_eq!(parent.name, "(none)".into());
        assert_eq!(parent.num, 0);

        assert_eq!(reader.stream_position().unwrap(), 20);
    }

    #[test]
    fn clip() {
        let mut reader = Cursor::new([
            0x49, 0x41, 0x53, 0x53, 0x00, 0x00, 0x00, 0x64, 0x46, 0x4c, 0x41, 0x54, 0x00, 0x5e,
            0x55, 0x6e, 0x74, 0x69, 0x74, 0x6c, 0x65, 0x64, 0x3a, 0x76, 0x69, 0x64, 0x65, 0x6f,
            0x53, 0x74, 0x69, 0x6c, 0x6c, 0x30, 0x30, 0x31, 0x00, 0x00, 0x66, 0x69, 0x6c, 0x65,
            0x6e, 0x61, 0x6d, 0x65, 0x00, 0x00, 0x76, 0x69, 0x64, 0x65, 0x6f, 0x53, 0x74, 0x69,
            0x6c, 0x6c, 0x00, 0x00, 0x69, 0x6d, 0x61, 0x67, 0x65, 0x00, 0x43, 0x3a, 0x5c, 0x55,
            0x73, 0x65, 0x72, 0x73, 0x5c, 0x76, 0x61, 0x6c, 0x65, 0x6e, 0x74, 0x69, 0x6e, 0x61,
            0x5c, 0x44, 0x6f, 0x63, 0x75, 0x6d, 0x65, 0x6e, 0x74, 0x73, 0x5c, 0x55, 0x6e, 0x74,
            0x69, 0x74, 0x6c, 0x65, 0x64, 0x2e, 0x74, 0x67, 0x61, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let iass = IncludeAsSubscene::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(iass.clips.len(), 1);
        assert_eq!(
            iass.clips[0],
            Flat {
                name: "Untitled:videoStill001".into(),
                source: "filename".into(),
                kind: "videoStill".into(),
                subkind: "image".into(),
                path: "C:\\Users\\valentina\\Documents\\Untitled.tga".into()
            }
        );

        assert_eq!(reader.stream_position().unwrap(), 108);
    }

    #[test]
    fn subscene() {
        let mut reader = Cursor::new([
            0x53, 0x55, 0x42, 0x53, 0x00, 0x00, 0x01, 0x70, 0x44, 0x6f, 0x77, 0x6e, 0x6c, 0x6f,
            0x61, 0x64, 0x73, 0x2f, 0x61, 0x61, 0x61, 0x2e, 0x6c, 0x78, 0x6f, 0x00, 0x6d, 0x65,
            0x73, 0x68, 0x00, 0x00, 0x49, 0x52, 0x45, 0x46, 0x00, 0x16, 0x73, 0x63, 0x65, 0x6e,
            0x65, 0x30, 0x30, 0x31, 0x00, 0x00, 0x53, 0x63, 0x65, 0x6e, 0x65, 0x00, 0x73, 0x63,
            0x65, 0x6e, 0x65, 0x00, 0x49, 0x52, 0x45, 0x46, 0x00, 0x26, 0x73, 0x68, 0x61, 0x64,
            0x65, 0x72, 0x46, 0x6f, 0x6c, 0x64, 0x65, 0x72, 0x30, 0x30, 0x38, 0x00, 0x4c, 0x69,
            0x67, 0x68, 0x74, 0x73, 0x00, 0x00, 0x73, 0x68, 0x61, 0x64, 0x65, 0x72, 0x46, 0x6f,
            0x6c, 0x64, 0x65, 0x72, 0x00, 0x00, 0x49, 0x52, 0x45, 0x46, 0x00, 0x2c, 0x73, 0x68,
            0x61, 0x64, 0x65, 0x72, 0x46, 0x6f, 0x6c, 0x64, 0x65, 0x72, 0x30, 0x30, 0x39, 0x00,
            0x45, 0x6e, 0x76, 0x69, 0x72, 0x6f, 0x6e, 0x6d, 0x65, 0x6e, 0x74, 0x73, 0x00, 0x00,
            0x73, 0x68, 0x61, 0x64, 0x65, 0x72, 0x46, 0x6f, 0x6c, 0x64, 0x65, 0x72, 0x00, 0x00,
            0x49, 0x52, 0x45, 0x46, 0x00, 0x1a, 0x63, 0x61, 0x6d, 0x65, 0x72, 0x61, 0x30, 0x30,
            0x33, 0x00, 0x43, 0x61, 0x6d, 0x65, 0x72, 0x61, 0x00, 0x00, 0x63, 0x61, 0x6d, 0x65,
            0x72, 0x61, 0x00, 0x00, 0x49, 0x52, 0x45, 0x46, 0x00, 0x2c, 0x72, 0x65, 0x6e, 0x64,
            0x65, 0x72, 0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x30, 0x32, 0x31, 0x00, 0x41, 0x6c,
            0x70, 0x68, 0x61, 0x20, 0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x00, 0x72, 0x65,
            0x6e, 0x64, 0x65, 0x72, 0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x00, 0x49, 0x52,
            0x45, 0x46, 0x00, 0x32, 0x72, 0x65, 0x6e, 0x64, 0x65, 0x72, 0x4f, 0x75, 0x74, 0x70,
            0x75, 0x74, 0x30, 0x32, 0x30, 0x00, 0x46, 0x69, 0x6e, 0x61, 0x6c, 0x20, 0x43, 0x6f,
            0x6c, 0x6f, 0x72, 0x20, 0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x00, 0x72, 0x65,
            0x6e, 0x64, 0x65, 0x72, 0x4f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x00, 0x49, 0x52,
            0x45, 0x46, 0x00, 0x26, 0x73, 0x68, 0x61, 0x64, 0x65, 0x72, 0x46, 0x6f, 0x6c, 0x64,
            0x65, 0x72, 0x30, 0x31, 0x31, 0x00, 0x4c, 0x69, 0x62, 0x72, 0x61, 0x72, 0x79, 0x00,
            0x73, 0x68, 0x61, 0x64, 0x65, 0x72, 0x46, 0x6f, 0x6c, 0x64, 0x65, 0x72, 0x00, 0x00,
            0x49, 0x52, 0x45, 0x46, 0x00, 0x22, 0x70, 0x6f, 0x6c, 0x79, 0x52, 0x65, 0x6e, 0x64,
            0x65, 0x72, 0x30, 0x30, 0x36, 0x00, 0x52, 0x65, 0x6e, 0x64, 0x65, 0x72, 0x00, 0x00,
            0x70, 0x6f, 0x6c, 0x79, 0x52, 0x65, 0x6e, 0x64, 0x65, 0x72, 0x00, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let subscene = Subscene::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(subscene.path, "Downloads/aaa.lxo".into());
        assert_eq!(subscene.name, "mesh".into());
        assert_eq!(subscene.item_references.len(), 8);
    }
}
