//! # lxoxide
//!
//! `lxoxide` is a Rust library for parsing and writing Luxology (`.lxo`) files.
//!
//! Luxology files are the native scene format used by Modo 3D software. They are based on the
//! IFF (Interchange File Format) chunk structure and are heavily derived from the
//! Lightwave (LWO) format.
//!
//! This crate provides a high-level abstraction for interacting with these files, allowing
//! you to extract scene geometry, item properties, animation data, and metadata, as well
//! as serialize modified scenes back to disk.
//!
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, Endian};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path as StdPath;
use std::str::FromStr;

pub mod animation;
pub mod geometry;
pub mod item;
pub mod meta;
pub mod primitives;
mod utils;

pub use primitives::{ChunkHeader, ID4};
use utils::write_chunk;

use animation::{Action, Envelope};
use geometry::layer::{
    BoundingBox, DiscontinuousVertexMap, Layer, Points, PolygonList, PolygonTagMapping,
    VertexEdgeMap, VertexMap, VertexMapParameter,
};
use geometry::trisurf::{
    TriSurfDataHeader, TriSurfGroupHeader, TriSurfTags, TriSurfTriangles, TriSurfVertexVectors,
    TriSurfVertices,
};
use item::{DataBlock, Item};
use meta::{
    ApplicationVersion, Audio, ChannelNames, Description, Encoding, IncludeAsSubscene, ItemTags,
    Preview, Reference, SceneTag, Subscene, Version,
};

/// File header will contain which type of file it is.
///
/// | ID4 | Name | Extension |
/// |---|---|---|
/// | LXOB | Scene (Luxology Object) | .lxo |
/// | LXPR | Preset | .lxp |
/// | LXPE | Environment Preset | .lxe |
/// | LXPM | Mesh Layer Preset | .lxl |
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[br(big, repr = u32)]
#[bw(big, repr = u32)]
pub enum Extension {
    Scene = 0x4c584f42,
    Preset = 0x4c585052,
    Environment = 0x4c585045,
    Mesh = 0x4c58504d,
}

#[derive(BinRead, Debug)]
#[br(magic = b"FORM")]
pub struct Header {
    #[br(big)]
    pub size: u32,

    #[br(big, map = Extension::from)]
    pub kind: Extension,
}

impl BinWrite for Header {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<()> {
        writer.write_all(b"FORM")?;
        self.size.write_be(writer)?;
        self.kind.write_be(writer)?;
        Ok(())
    }
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

    #[error("Chunk {kind} has odd size {size}, data must be padded to even bytes")]
    OddChunkSize { kind: ID4, size: u32 },

    #[error("Chunk {kind} consumed {consumed} bytes, expected {expected}")]
    ChunkBoundaryMismatch {
        kind: String,
        expected: u32,
        consumed: u64,
    },

    #[error("File type not supported")]
    NonSupportedExtension,

    #[error("Buffer is too short for the data to be parsed")]
    BufferTooShort,

    #[error("Strings must be null terminated")]
    MissingNullTerminator,

    #[error("Bytes must be aligned to even number")]
    UnalignedBytes,

    #[error("Invalid Channel Data Mask")]
    InvalidChannelDataMask,

    #[error("Non supported Channel Vector data type")]
    ChannelVectorArray,

    #[error("Missing LAYR")]
    MissingLayer,

    #[error("Missing PNTS")]
    MissingPoints,

    #[error("No previous parsed POLS chunk")]
    MissingPolygonsList,

    // Any 3SRF must come after a 3GRP
    #[error("Missing Trisurface Group Header 3GRP")]
    MissingTriSurfGroupHeader,

    // Any VRTS, TRIS, VVEC and TTGS must come after a 3SRF
    #[error("Missing Trisurface Data Header 3SRF")]
    MissingTriSurfDataHeader,

    #[error("Invalid XMAN mode {mode}, expected 2 or 3")]
    InvalidXManMode { mode: u32 },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    BinRead(#[from] binrw::Error),
}

/// Enumerator of all known chunks as well as Unknown to store unhandled chunks
pub enum Chunk {
    Preview(Preview),
    Description(Description),
    Version(Version),
    ApplicationVersion(ApplicationVersion),
    Encoding(Encoding),
    IncludeAsSubscene(IncludeAsSubscene),
    Subscene(Subscene),
    Reference(Reference),
    SceneTag(SceneTag),
    ItemTags(ItemTags),
    ChannelNames(ChannelNames),
    Layer(Box<Layer>),
    Points(Points),
    BoundingBox(BoundingBox),
    VertexMapParameter(VertexMapParameter),
    VertexMap(VertexMap),
    VertexEdgeMap(VertexEdgeMap),
    PolygonList(PolygonList),
    DiscontinuousVertexMap(DiscontinuousVertexMap),
    PolygonTagMapping(PolygonTagMapping),
    TriSurfGroupHeader(TriSurfGroupHeader),
    TriSurfDataHeader(TriSurfDataHeader),
    TriSurfVertices(TriSurfVertices),
    TriSurfTriangles(TriSurfTriangles),
    TriSurfVertexVectors(TriSurfVertexVectors),
    TriSurfTags(TriSurfTags),
    Item(Box<Item>),
    Envelope(Envelope),
    Action(Action),
    DataBlock(DataBlock),
    Audio(Audio),
    Unknown((ChunkHeader, Vec<u8>)),
}

pub struct LuxologyFile {
    header: Header,
    chunks: Vec<Chunk>,
}

impl LuxologyFile {
    fn parse<R: Read + Seek>(reader: &mut R, total_size: u64) -> Result<LuxologyFile, ParseError> {
        let header: Header = reader.read_be()?;

        // Check that the reported size of content, matches file size
        // Modo will however happily go ahead and just parse the first file if we concat
        // two files. Causing the second FORM to just be dropped when saving.
        if total_size != header.size as u64 + 8 {
            return Err(ParseError::SizeMismatch);
        }

        let mut chunks: Vec<Chunk> = vec![];

        loop {
            let chunk_start_position = reader.stream_position()? as i64;
            let chunk_header = match ChunkHeader::read_be(reader) {
                Ok(h) => h,
                Err(e) => {
                    if e.is_eof() {
                        break;
                    }
                    return Err(e.into());
                }
            };

            let remaining_bytes = total_size - chunk_start_position as u64;
            if (chunk_header.size as u64 + 8) > remaining_bytes {
                return Err(ParseError::InvalidSize);
            }

            if chunk_header.size % 2 != 0 {
                return Err(ParseError::OddChunkSize {
                    kind: chunk_header.kind,
                    size: chunk_header.size,
                });
            }

            // todo: We should likely make all read_be_args accept size in a more uniform way
            match chunk_header.kind.as_str() {
                "PRVW" => chunks.push(Chunk::Preview(Preview::read_be_args(
                    reader,
                    (chunk_header.size,),
                )?)),
                "DESC" => chunks.push(Chunk::Description(Description::read_be(reader)?)),
                "VRSN" => chunks.push(Chunk::Version(Version::read_be(reader)?)),
                "APPV" => chunks.push(Chunk::ApplicationVersion(ApplicationVersion::read_be(
                    reader,
                )?)),
                "ENCO" => chunks.push(Chunk::Encoding(Encoding::read_be(reader)?)),
                "IASS" => chunks.push(Chunk::IncludeAsSubscene(IncludeAsSubscene::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "SUBS" => chunks.push(Chunk::Subscene(Subscene::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "XREF" => chunks.push(Chunk::Reference(Reference::read_be(reader)?)),
                "STAG" => chunks.push(Chunk::SceneTag(SceneTag::read_be(reader)?)),
                "TAGS" => chunks.push(Chunk::ItemTags(ItemTags::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "CHNM" => chunks.push(Chunk::ChannelNames(ChannelNames::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "LAYR" => chunks.push(Chunk::Layer(Box::new(Layer::read_be(reader)?))),
                "PNTS" => chunks.push(Chunk::Points(Points::read_be_args(
                    reader,
                    (chunk_header.size / 12,),
                )?)),
                "BBOX" => chunks.push(Chunk::BoundingBox(BoundingBox::read_be(reader)?)),
                "VMPA" => chunks.push(Chunk::VertexMapParameter(VertexMapParameter::read_be(
                    reader,
                )?)),
                "VMAP" => chunks.push(Chunk::VertexMap(VertexMap::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "VMED" => chunks.push(Chunk::VertexEdgeMap(VertexEdgeMap::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "POLS" => chunks.push(Chunk::PolygonList(PolygonList::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "VMAD" => chunks.push(Chunk::DiscontinuousVertexMap(
                    DiscontinuousVertexMap::read_be_args(reader, chunk_header.size)?,
                )),
                "PTAG" => chunks.push(Chunk::PolygonTagMapping(PolygonTagMapping::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "3GRP" => chunks.push(Chunk::TriSurfGroupHeader(TriSurfGroupHeader::read_be(
                    reader,
                )?)),
                "3SRF" => chunks.push(Chunk::TriSurfDataHeader(TriSurfDataHeader::read_be(
                    reader,
                )?)),
                "VRTS" => chunks.push(Chunk::TriSurfVertices(TriSurfVertices::read_be_args(
                    reader,
                    (chunk_header.size / 12,),
                )?)),
                "TRIS" => chunks.push(Chunk::TriSurfTriangles(TriSurfTriangles::read_be_args(
                    reader,
                    (chunk_header.size / 12,),
                )?)),
                "VVEC" => chunks.push(Chunk::TriSurfVertexVectors(
                    TriSurfVertexVectors::read_be_args(reader, chunk_header.size)?,
                )),
                "TTGS" => chunks.push(Chunk::TriSurfTags(TriSurfTags::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "ITEM" => chunks.push(Chunk::Item(Box::new(Item::read_be_args(
                    reader,
                    chunk_header.size,
                )?))),
                "ENVL" => chunks.push(Chunk::Envelope(Envelope::read_be(reader)?)),
                "ACTN" => chunks.push(Chunk::Action(Action::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "DATA" => chunks.push(Chunk::DataBlock(DataBlock::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                "AANI" => chunks.push(Chunk::Audio(Audio::read_be_args(
                    reader,
                    chunk_header.size,
                )?)),
                _ => {
                    let mut chunk_data: Vec<u8> = Vec::with_capacity(chunk_header.size as usize);
                    reader.read_exact(&mut chunk_data)?;
                    chunks.push(Chunk::Unknown((chunk_header, chunk_data)));
                }
            }

            // additional safety checks, to catch if any parser did not consume the full chunk
            // payload
            let consumed = reader.stream_position()? - chunk_start_position as u64 - 8;
            if consumed != chunk_header.size as u64 {
                return Err(ParseError::ChunkBoundaryMismatch {
                    kind: chunk_header.kind.to_string(),
                    expected: chunk_header.size,
                    consumed,
                });
            }
        }

        Ok(LuxologyFile { header, chunks })
    }

    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<LuxologyFile, ParseError> {
        let start = reader.stream_position()?;
        let total_size = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(start))?;
        Self::parse(reader, total_size - start)
    }

    pub fn from_path<P: AsRef<StdPath>>(path: P) -> Result<LuxologyFile, ParseError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        Self::from_reader(&mut reader)
    }

    pub fn to_writer<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        let mut buf = Vec::new();
        let mut bw = Cursor::new(&mut buf);

        for chunk in &self.chunks {
            match chunk {
                Chunk::Preview(preview) => {
                    write_chunk(&mut bw, ID4::from_str("PRVW").unwrap(), &preview)?
                }
                Chunk::Description(desc) => {
                    write_chunk(&mut bw, ID4::from_str("DESC").unwrap(), &desc)?
                }
                Chunk::Version(version) => {
                    write_chunk(&mut bw, ID4::from_str("VRSN").unwrap(), &version)?
                }
                Chunk::ApplicationVersion(application_version) => write_chunk(
                    &mut bw,
                    ID4::from_str("APPV").unwrap(),
                    &application_version,
                )?,
                Chunk::Encoding(encoding) => {
                    write_chunk(&mut bw, ID4::from_str("ENCO").unwrap(), &encoding)?
                }
                Chunk::IncludeAsSubscene(iass) => {
                    write_chunk(&mut bw, ID4::from_str("IASS").unwrap(), &iass)?
                }
                Chunk::Subscene(subscene) => {
                    write_chunk(&mut bw, ID4::from_str("SUBS").unwrap(), &subscene)?
                }
                Chunk::Reference(xref) => {
                    write_chunk(&mut bw, ID4::from_str("XREF").unwrap(), &xref)?
                }
                Chunk::SceneTag(scene_tag) => {
                    write_chunk(&mut bw, ID4::from_str("STAG").unwrap(), &scene_tag)?
                }
                Chunk::ItemTags(item_tags) => {
                    write_chunk(&mut bw, ID4::from_str("TAGS").unwrap(), &item_tags)?
                }
                Chunk::ChannelNames(channel_names) => {
                    write_chunk(&mut bw, ID4::from_str("CHNM").unwrap(), &channel_names)?
                }
                Chunk::Layer(layer) => {
                    write_chunk(&mut bw, ID4::from_str("LAYR").unwrap(), layer)?
                }
                Chunk::Points(points) => {
                    write_chunk(&mut bw, ID4::from_str("PNTS").unwrap(), &points)?
                }
                Chunk::BoundingBox(bbox) => {
                    write_chunk(&mut bw, ID4::from_str("BBOX").unwrap(), &bbox)?
                }
                Chunk::VertexMapParameter(vmap_params) => {
                    write_chunk(&mut bw, ID4::from_str("VMPA").unwrap(), &vmap_params)?
                }
                Chunk::VertexMap(vmap) => {
                    write_chunk(&mut bw, ID4::from_str("VMAP").unwrap(), &vmap)?
                }
                Chunk::VertexEdgeMap(vmed) => {
                    write_chunk(&mut bw, ID4::from_str("VMED").unwrap(), &vmed)?
                }
                Chunk::PolygonList(pols) => {
                    write_chunk(&mut bw, ID4::from_str("POLS").unwrap(), &pols)?
                }
                Chunk::DiscontinuousVertexMap(vmad) => {
                    write_chunk(&mut bw, ID4::from_str("VMAD").unwrap(), &vmad)?
                }
                Chunk::PolygonTagMapping(ptag) => {
                    write_chunk(&mut bw, ID4::from_str("PTAG").unwrap(), &ptag)?
                }
                Chunk::TriSurfGroupHeader(trisurf_group) => {
                    write_chunk(&mut bw, ID4::from_str("3GRP").unwrap(), &trisurf_group)?
                }
                Chunk::TriSurfDataHeader(trisurf_data) => {
                    write_chunk(&mut bw, ID4::from_str("3SRF").unwrap(), &trisurf_data)?
                }
                Chunk::TriSurfVertices(vertices) => {
                    write_chunk(&mut bw, ID4::from_str("VRTS").unwrap(), &vertices)?
                }
                Chunk::TriSurfTriangles(triangles) => {
                    write_chunk(&mut bw, ID4::from_str("TRIS").unwrap(), &triangles)?
                }
                Chunk::TriSurfVertexVectors(vvec) => {
                    write_chunk(&mut bw, ID4::from_str("VVEC").unwrap(), &vvec)?
                }
                Chunk::TriSurfTags(ttag) => {
                    write_chunk(&mut bw, ID4::from_str("TTAG").unwrap(), &ttag)?
                }
                Chunk::Item(item) => {
                    write_chunk(&mut bw, ID4::from_str("ITEM").unwrap(), item)?
                }
                Chunk::Envelope(envelope) => {
                    write_chunk(&mut bw, ID4::from_str("ENVL").unwrap(), &envelope)?
                }
                Chunk::Action(action) => {
                    write_chunk(&mut bw, ID4::from_str("ACTN").unwrap(), &action)?
                }
                Chunk::DataBlock(data) => {
                    write_chunk(&mut bw, ID4::from_str("DATA").unwrap(), &data)?
                }
                Chunk::Audio(audio) => {
                    write_chunk(&mut bw, ID4::from_str("AANI").unwrap(), &audio)?
                }
                Chunk::Unknown((header, data)) => write_chunk(&mut bw, header.kind, &data)?,
            }
        }

        // Write FORM header with computed size
        // We add 4, as the extension is also counted for size,
        let size = (buf.len() + 4) as u32;
        let header = Header {
            size,
            kind: self.header.kind,
        };
        header.write_be(writer)?;
        writer.write_all(&buf)?;
        Ok(())
    }

    pub fn to_path<P: AsRef<StdPath>>(&self, path: P) -> BinResult<()> {
        let file = File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);
        self.to_writer(&mut writer)
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

        assert_eq!(header.kind, Extension::Scene);
    }

    #[test]
    #[should_panic]
    fn parse_invalid_magic_header() {
        let mut reader = Cursor::new(b"BAD \x00\x00\x61\x6aLXOB");
        let header: Header = reader.read_be().unwrap();

        assert_eq!(header.kind, Extension::Scene);
    }
}
