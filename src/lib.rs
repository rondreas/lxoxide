use binrw::{BinRead, BinReaderExt, NullString};
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Seek};
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
    DiscontinousVertexMap, Layer, Points, BoundingBox, PolygonList, PolygonTagMapping, VertexMap,
    VertexMapParameter,
};
use geometry::trisurf::TriSurfGroupHeader;
use item::{Item, DataBlock};
use media::Audio;
use meta::{IncludeAsSubscene, ChannelNames, ItemTags, Description};

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

    pub description: Option<Description>,
    pub version: Option<Version>,
    pub application_version: Option<ApplicationVersion>,
    pub encoding: Option<Encoding>,

    // even scenes which does not include subscenes, tend to have this chunk
    pub included_subscene: Option<IncludeAsSubscene>,

    // todo: rewrite these chunks to just be a Vec<NullString>, we don't need structs for them
    pub item_tags: Option<ItemTags>,
    pub channel_names: Option<ChannelNames>,

    // geometry is a bit more complex. as first we will have a chunk saying
    // this is a new layer/trisurf. followed by chunks that make it's data.
    pub layers: Vec<Layer>,
    pub trisurfs: Vec<TriSurfGroupHeader>,

    pub items: Vec<Item>,
    pub envelopes: Vec<Envelope>,
    pub actions: Vec<Action>,

    pub data_blocks: Vec<DataBlock>,

    // At the end files tend to have audio
    pub audio: Option<Audio>,
}

impl LuxologyFile {
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

        let mut description = None;
        let mut version = None;
        let mut application_version = None;
        let mut encoding = None;
        let mut included_subscene = None;
        let mut item_tags = None;
        let mut channel_names = None;

        let mut layers = vec![];
        let mut trisurfs = vec![];
        let mut items = vec![];
        let mut envelopes = vec![];
        let mut actions = vec![];
        let mut data_blocks = vec![];

        let mut audio = None;

        loop {
            let chunk_start_position = reader.stream_position()?;
            let chunk_header = match ChunkHeader::read_be(&mut reader) {
                Ok(h) => h,
                Err(e) => {
                    if e.is_eof() {
                        break;
                    }
                    return Err(e.into());
                }
            };

            match chunk_header.kind.as_str() {
                "DESC" => description = Some(Description::read_be(&mut reader)?),
                "VRSN" => version = Some(Version::read_be(&mut reader)?),
                "APPV" => application_version = Some(ApplicationVersion::read_be(&mut reader)?),
                "ENCO" => encoding = Some(Encoding::read_be(&mut reader)?),
                "IASS" => included_subscene = Some(IncludeAsSubscene::read_be(&mut reader)?),
                "TAGS" => item_tags = Some(ItemTags::read_be_args(&mut reader, chunk_header.size)?),
                "CHNM" => channel_names = Some(ChannelNames::read_be_args(&mut reader, chunk_header.size)?),
                "LAYR" => layers.push(Layer::read_be(&mut reader)?),
                "PNTS" => {
                    match layers.last_mut() {
                        Some(layer) => layer.points = Some(Points::read_be_args(&mut reader, (chunk_header.size / 12,))?),
                        _ => eprintln!("Orphan points")
                    }
                },
                "BBOX" => {
                    match layers.last_mut() {
                        Some(layer) => layer.bounds = Some(BoundingBox::read_be(&mut reader)?),
                        _ => eprintln!("Orphan bounds")
                    }
                },
                "VMPA" => {
                    match layers.last_mut() {
                        Some(layer) => layer.vertex_map_parameters.push(VertexMapParameter::read_be(&mut reader)?),
                        _ => eprintln!("Orphan vertex map")
                    }
                },
                "VMAP" => {
                    match layers.last_mut() {
                        Some(layer) => layer.vertex_maps.push(VertexMap::read_be_args(&mut reader, chunk_header.size)?),
                        _ => eprintln!("Orphan vertex map")
                    }
                },
                "POLS" => {
                    match layers.last_mut() {
                        Some(layer) => layer.polygons.push(PolygonList::read_be_args(&mut reader, chunk_header.size)?),
                        _ => eprintln!("Orphan polygon list")
                    }
                },
                "VMAD" => {
                    match layers.last_mut() {
                        Some(layer) => layer.discontinous_vertex_maps.push(DiscontinousVertexMap::read_be_args(&mut reader, chunk_header.size)?),
                        _ => eprintln!("Orphan discontinous vertex map")
                    }
                },
                "PTAG" => {
                    match layers.last_mut() {
                        Some(layer) => layer.polygon_tags.push(PolygonTagMapping::read_be_args(&mut reader, chunk_header.size)?),
                        _ => eprintln!("Orphan polygon tag mapping")
                    }
                },
                "3GRP" => trisurfs.push(TriSurfGroupHeader::read_be(&mut reader)?),
                "ITEM" => items.push(Item::read_be_args(&mut reader, chunk_header.size)?),
                "ENVL" => envelopes.push(Envelope::read_be(&mut reader)?),
                "ACTN" => actions.push(Action::read_be_args(&mut reader, chunk_header.size)?),
                "DATA" => data_blocks.push(DataBlock::read_be_args(&mut reader, chunk_header.size)?),
                "AANI" => audio = Some(Audio::read_be_args(&mut reader, chunk_header.size)?),
                _ => {
                    // just eprint? or keep in vec<unknowns>?
                    eprintln!(
                        "Unknown Chunk {}, pos: {}, size: {}",
                        chunk_header.kind,
                        chunk_start_position,
                        chunk_header.size + 8  // size of header + data
                    );

                    reader.seek_relative(chunk_header.size as i64)?
                },
            }
        }

        Ok(LuxologyFile{
            header,
            description,
            version,
            application_version,
            encoding,
            included_subscene,
            item_tags,
            channel_names,
            layers,
            trisurfs,
            items,
            envelopes,
            actions,
            data_blocks,
            audio
        })
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
