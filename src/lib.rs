use binrw::{BinRead, BinReaderExt, NullString};
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path as StdPath;

pub mod animation;
pub mod geometry;
pub mod item;
pub mod media;
pub mod meta;
pub mod primitives;
pub mod utils;

pub use primitives::{ChunkHeader, ID4};

use animation::{Action, Envelope};
use geometry::layer::{
    DiscontinousVertexMap, Layer, Points, PolygonList, PolygonTagMapping, VertexMap,
    VertexMapParameter,
};
use item::Item;
use media::Audio;
use meta::{ChannelNames, ItemTags};

#[derive(BinRead, Debug, Clone, Copy, PartialEq, Eq)]
#[br(repr = u32)]
pub enum Extension {
    LXOB = 0x4c584f42, // scene file
    LXPR = 0x4c585052, // preset assembly
    LXPE = 0x4c585045, // preset environment
    LXPM = 0x4c58504d, // preset item
}

impl TryFrom<u32> for Extension {
    type Error = ParseError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x4c584f42 => Ok(Extension::LXOB),
            0x4c585052 => Ok(Extension::LXPR),
            0x4c585045 => Ok(Extension::LXPE),
            0x4c58504d => Ok(Extension::LXPM),
            _ => Err(ParseError::InvalidID4),
        }
    }
}

impl Extension {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x4c584f42 => Some(Extension::LXOB),
            0x4c585052 => Some(Extension::LXPR),
            0x4c585045 => Some(Extension::LXPE),
            0x4c58504d => Some(Extension::LXPM),
            _ => None,
        }
    }
}

#[derive(BinRead, Debug)]
#[br(magic = b"FORM")]
pub struct Header {
    #[br(big)]
    pub size: u32,

    #[br(big, map = Extension::from)]
    pub kind: Extension,
}

// Enum for all chunks, storing unknown with information to more easy check hexdump
#[derive(Debug)]
pub enum Chunk {
    VRSN(Version),
    APPV(ApplicationVersion),
    ENCO(Encoding),
    TAGS(ItemTags),
    CHNM(ChannelNames),
    LAYR(Layer),
    PNTS(Points),
    VMPA(VertexMapParameter),
    VMAP(VertexMap),
    POLS(PolygonList),
    VMAD(DiscontinousVertexMap),
    PTAG(PolygonTagMapping),
    ITEM(Item),
    ENVL(Envelope),
    ACTN(Action),
    AANI(Audio),
    Unknown { kind: ID4, position: u64, size: u32 },
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("IFF files must start with FORM")]
    InvalidMagicNumber,

    #[error("ID4 must be 4 printable ASCII characters")]
    InvalidID4,

    #[error("File size does not match reported size in header")]
    SizeMismatch,

    #[error("Invalid size for fixed size chunk data")]
    InvalidSize,

    #[error("File type not supported")]
    NonSupportedExtension,

    #[error("Buffer is too short for the data to be parsed")]
    BufferTooShort,

    #[error("Strings must be null terminated")]
    MissingNullTerminator,

    #[error("Bytes must be aligned to even number")]
    UnalignedBytes,

    #[error("Non supported Channel Vector data type")]
    ChannelVectorArray,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    BinRead(#[from] binrw::Error),
}

pub struct LuxologyFile {
    pub header: Header,
    pub chunks: Vec<Chunk>,
}

impl LuxologyFile {
    pub fn new(header: Header, chunks: Vec<Chunk>) -> LuxologyFile {
        LuxologyFile { header, chunks }
    }

    pub fn from_path<P: AsRef<StdPath>>(path: P) -> Result<LuxologyFile, ParseError> {
        let file = File::open(path)?;
        let meta = file.metadata()?;
        let mut reader = BufReader::new(file);

        let header: Header = reader.read_be().unwrap();

        // Check that the reported size of content, matches file size
        // Modo will however happily go ahead and just parse the first file if we concat
        // two files. Causing the second FORM to just be dropped when saving.
        if meta.len() != header.size as u64 + 8 {
            return Err(ParseError::InvalidSize);
        }

        let chunks = Self::parse_chunks(&mut reader)?;

        Ok(LuxologyFile::new(header, chunks))
    }

    fn parse_chunks<R: Read + Seek>(reader: &mut R) -> Result<Vec<Chunk>, ParseError> {
        let mut chunks = Vec::new();
        loop {
            let header = match ChunkHeader::read_be(reader) {
                Ok(h) => h,
                Err(e) => {
                    if e.is_eof() {
                        break;
                    }
                    return Err(e.into());
                }
            };

            match header.kind.as_str() {
                "VRSN" => {
                    let version: Version = reader.read_be().unwrap();
                    chunks.push(Chunk::VRSN(version));
                }
                "APPV" => {
                    let version: ApplicationVersion = reader.read_be().unwrap();
                    chunks.push(Chunk::APPV(version));
                }
                "ENCO" => {
                    let encoding: Encoding = reader.read_be().unwrap();
                    chunks.push(Chunk::ENCO(encoding));
                }
                "TAGS" => {
                    let tags = ItemTags::read_be_args(reader, header.size).unwrap();
                    chunks.push(Chunk::TAGS(tags));
                }
                "CHNM" => {
                    let channel_names = ChannelNames::read_be_args(reader, header.size).unwrap();
                    chunks.push(Chunk::CHNM(channel_names));
                }
                "LAYR" => {
                    let layer: Layer = reader.read_be().unwrap();
                    chunks.push(Chunk::LAYR(layer));
                }
                "PNTS" => {
                    let points = Points::read_args(reader, (header.size / 12,)).unwrap();
                    chunks.push(Chunk::PNTS(points));
                }
                "VMPA" => {
                    let vertex_params = VertexMapParameter::read_be(reader)?;
                    chunks.push(Chunk::VMPA(vertex_params));
                }
                "VMAP" => {
                    let vertex_map = VertexMap::read_be_args(reader, header.size)?;
                    chunks.push(Chunk::VMAP(vertex_map));
                }
                "POLS" => {
                    let polygon_list = PolygonList::read_args(reader, header.size).unwrap();
                    chunks.push(Chunk::POLS(polygon_list));
                }
                "VMAD" => {
                    let vmad = DiscontinousVertexMap::read_be_args(reader, header.size)?;
                    chunks.push(Chunk::VMAD(vmad));
                }
                "PTAG" => {
                    let ptag = PolygonTagMapping::read_be_args(reader, header.size)?;
                    chunks.push(Chunk::PTAG(ptag));
                }
                "ITEM" => {
                    let item = Item::read_args(reader, header.size)?;
                    chunks.push(Chunk::ITEM(item));
                }
                "ENVL" => {
                    let envelope = Envelope::read_be(reader)?;
                    chunks.push(Chunk::ENVL(envelope));
                }
                "ACTN" => {
                    let action = Action::read_be_args(reader, header.size)?;
                    chunks.push(Chunk::ACTN(action));
                }
                "AANI" => {
                    let audio = Audio::read_be_args(reader, header.size)?;
                    chunks.push(Chunk::AANI(audio));
                }
                _ => {
                    // push the unknown chunk, with offset and size so we can quickly find it
                    // with hex dump, like `xxd -s position -l size ./path/to/file.lxo`
                    chunks.push(Chunk::Unknown {
                        kind: header.kind,
                        position: reader.stream_position().unwrap() - 8,
                        size: header.size + 8,
                    });

                    reader.seek(SeekFrom::Current(header.size as i64))?;
                }
            }
        }

        Ok(chunks)
    }
}

#[derive(BinRead, Debug, PartialEq)]
#[br(big)]
pub struct Version {
    major: u32,

    minor: u32,

    #[br(align_after = 2)]
    application: NullString,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{} {}", self.major, self.minor, self.application)
    }
}

#[derive(BinRead, Debug, PartialEq)]
#[br(big)]
pub struct ApplicationVersion {
    base: u32,
    major: u32,
    minor: u32,
    build: u32,

    #[br(align_after = 2)]
    application: NullString,
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

#[derive(BinRead, Debug, PartialEq)]
#[br(big, repr = u32)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_lxo_header() {
        let mut reader = Cursor::new(b"FORM\x00\x00\x61\x6aLXOB");
        let header: Header = reader.read_be().unwrap();

        assert_eq!(header.kind, Extension::LXOB);
    }

    #[test]
    #[should_panic]
    fn parse_invalid_magic_header() {
        let mut reader = Cursor::new(b"BAD \x00\x00\x61\x6aLXOB");
        let header: Header = reader.read_be().unwrap();

        assert_eq!(header.kind, Extension::LXOB);
    }

    #[test]
    fn parse_version_chunk() {
        let mut reader = Cursor::new(b"VRSN\x00\x00\x00\x22\x00\x00\x00\x10\x00\x00\x00\x00\x6e\x65\x78\x75\x73\x20\x32\x30\x30\x30\x20\x62\x79\x20\x54\x68\x65\x20\x46\x6f\x75\x6e\x64\x72\x79\x00");

        let _: ChunkHeader = reader.read_be().unwrap();

        let expected = Version {
            major: 16,
            minor: 0,
            application: "nexus 2000 by The Foundry".into(),
        };
        let result: Version = reader.read_be().unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn parse_application_version_chunk() {
        let mut reader = Cursor::new(b"APPV\x00\x00\x00\x1c\x00\x00\x07\xd0\x00\x00\x07\xd0\x00\x00\x00\x00\x00\x0a\x17\xc6\x4d\x6f\x64\x6f\x20\x31\x36\x2e\x30\x76\x31\x00");

        let _: ChunkHeader = reader.read_be().unwrap();

        let expected = ApplicationVersion {
            base: 2000, // this version matches the nexus.dll that likely was used to save
            major: 2000,
            minor: 0,
            build: 661446,
            application: "Modo 16.0v1".into(),
        };

        let result: ApplicationVersion = reader.read_be().unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn parse_encoding_chunk() {
        let mut reader = Cursor::new(b"ENCO\x00\x00\x00\x04\x00\x00\x00\x02");
        let _: ChunkHeader = reader.read_be().unwrap();
        let encoding: Encoding = reader.read_be().unwrap();
        assert_eq!(encoding, Encoding::Utf8);
    }

    // todo: get test to work... can't for life of me match the error
    // #[test]
    // fn parse_invalid_encoding_chunk() {
    //     let mut reader = Cursor::new(b"ENCO\x00\x00\x00\x04\x00\x00\x0c\x02");
    //     let _: ChunkHeader = reader.read_be().unwrap();
    //     let r = reader.read_be::<Encoding>();
    //     match r {
    //         Ok(_) => panic!("Expected an error but parsing succeeded!"),
    //         Err(e) => {
    //             let mut err = &e;
    //             while let binrw::Error::Backtrace(bt) = err {
    //                 err = &bt.error;
    //             }
    //
    //             match err {
    //                 Err(binrw::Error::NoVariantMatch{pos: _}) => { /* Test pass */ }
    //                 _ => panic!("wtf..."),
    //             }
    //         },
    //     }
    // }
}
