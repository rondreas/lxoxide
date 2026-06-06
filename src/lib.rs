use binrw::{BinRead, BinReaderExt};
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
    BoundingBox, DiscontinousVertexMap, Layer, Points, PolygonGroup, PolygonList,
    PolygonTagMapping, VertexEdgeMap, VertexMap, VertexMapParameter,
};
use geometry::trisurf::{
    TriSurfDataHeader, TriSurfGroupHeader, TriSurfTags, TriSurfTriangles, TriSurfVertexVectors,
    TriSurfVertices,
};
use item::{DataBlock, Item};
use media::Audio;
use meta::{
    ApplicationVersion, ChannelNames, Description, Encoding, IncludeAsSubscene, ItemTags, Preview,
    Reference, Subscene, Version,
};

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
            _ => Err(ParseError::NonSupportedExtension),
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

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    BinRead(#[from] binrw::Error),
}

pub struct LuxologyFile {
    pub header: Header,

    // Optional thumbnail for the scene
    pub preview: Option<Preview>,

    pub description: Option<Description>,
    pub version: Option<Version>,
    pub application_version: Option<ApplicationVersion>,
    pub encoding: Option<Encoding>,

    // even scenes which does not include subscenes, tend to have this chunk
    pub included_subscene: Option<IncludeAsSubscene>,

    // references are included as subscenes and references, SUBS and XREF chunks
    pub subscenes: Vec<Subscene>,
    pub references: Vec<Reference>,

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
    fn get_last_mut_trisurf(
        trisurfs: &mut [TriSurfGroupHeader],
    ) -> Result<&mut TriSurfDataHeader, ParseError> {
        let trisurf = trisurfs
            .last_mut()
            .ok_or(ParseError::MissingTriSurfGroupHeader)?
            .trisurfaces
            .last_mut()
            .ok_or(ParseError::MissingTriSurfDataHeader)?;

        Ok(trisurf)
    }

    pub fn from_path<P: AsRef<StdPath>>(path: P) -> Result<LuxologyFile, ParseError> {
        let file = File::open(path)?;
        let meta = file.metadata()?;
        let mut reader = BufReader::new(file);

        let header: Header = reader.read_be()?;

        // Check that the reported size of content, matches file size
        // Modo will however happily go ahead and just parse the first file if we concat
        // two files. Causing the second FORM to just be dropped when saving.
        if meta.len() != header.size as u64 + 8 {
            return Err(ParseError::SizeMismatch);
        }

        let mut preview = None;
        let mut description = None;
        let mut version = None;
        let mut application_version = None;
        let mut encoding = None;
        let mut included_subscene = None;
        let mut subscenes = vec![];
        let mut references = vec![];
        let mut item_tags = None;
        let mut channel_names = None;

        let mut layers = vec![];
        let mut trisurfs = vec![];
        let mut items = vec![];
        let mut envelopes = vec![];
        let mut actions = vec![];
        let mut data_blocks = vec![];

        let mut audio = None;

        // While parsing layers, we keep track of the last polygon kind as we need to match
        // VMAD and PTAG to the previous POLS
        let mut last_pols_kind = ID4::new(*b"UNKN");

        loop {
            let chunk_start_position = reader.stream_position()? as i64;
            let chunk_header = match ChunkHeader::read_be(&mut reader) {
                Ok(h) => h,
                Err(e) => {
                    if e.is_eof() {
                        break;
                    }
                    return Err(e.into());
                }
            };

            let remaining_bytes = meta.len() - chunk_start_position as u64;
            if (chunk_header.size as u64 + 8) > remaining_bytes {
                return Err(ParseError::InvalidSize);
            }

            if chunk_header.size % 2 != 0 {
                return Err(ParseError::OddChunkSize {
                    kind: chunk_header.kind,
                    size: chunk_header.size,
                });
            }

            match chunk_header.kind.as_str() {
                "PRVW" => preview = Some(Preview::read_be_args(&mut reader, (chunk_header.size,))?),
                "DESC" => description = Some(Description::read_be(&mut reader)?),
                "VRSN" => version = Some(Version::read_be(&mut reader)?),
                "APPV" => application_version = Some(ApplicationVersion::read_be(&mut reader)?),
                "ENCO" => encoding = Some(Encoding::read_be(&mut reader)?),
                "IASS" => {
                    included_subscene = Some(IncludeAsSubscene::read_be_args(
                        &mut reader,
                        chunk_header.size,
                    )?)
                }
                "SUBS" => subscenes.push(Subscene::read_be_args(&mut reader, chunk_header.size)?),
                "XREF" => references.push(Reference::read_be(&mut reader)?),
                "TAGS" => item_tags = Some(ItemTags::read_be_args(&mut reader, chunk_header.size)?),
                "CHNM" => {
                    channel_names =
                        Some(ChannelNames::read_be_args(&mut reader, chunk_header.size)?)
                }
                "LAYR" => layers.push(Layer::read_be(&mut reader)?),
                "PNTS" => {
                    layers
                        .last_mut()
                        .ok_or(ParseError::MissingLayer)?
                        .geometry
                        .points = Some(Points::read_be_args(
                        &mut reader,
                        (chunk_header.size / 12,),
                    )?);
                }
                "BBOX" => {
                    layers
                        .last_mut()
                        .ok_or(ParseError::MissingLayer)?
                        .geometry
                        .bounds = Some(BoundingBox::read_be(&mut reader)?);
                }
                "VMPA" => {
                    layers
                        .last_mut()
                        .ok_or(ParseError::MissingLayer)?
                        .geometry
                        .vertex_map_parameters
                        .push(VertexMapParameter::read_be(&mut reader)?);
                }
                "VMAP" => {
                    layers
                        .last_mut()
                        .ok_or(ParseError::MissingLayer)?
                        .geometry
                        .vertex_maps
                        .push(VertexMap::read_be_args(&mut reader, chunk_header.size)?);
                }
                "VMED" => {
                    layers
                        .last_mut()
                        .ok_or(ParseError::MissingLayer)?
                        .geometry
                        .vertex_edge_maps
                        .push(VertexEdgeMap::read_be_args(&mut reader, chunk_header.size)?);
                }
                "POLS" => {
                    let polygons = PolygonList::read_be_args(&mut reader, chunk_header.size)?;
                    last_pols_kind = polygons.kind;
                    let geometry = &mut layers.last_mut().ok_or(ParseError::MissingLayer)?.geometry;
                    geometry.points.as_ref().ok_or(ParseError::MissingPoints)?;
                    geometry
                        .polygons
                        .insert(polygons.kind, PolygonGroup::new(polygons));
                }
                "VMAD" => {
                    layers
                        .last_mut()
                        .ok_or(ParseError::MissingLayer)?
                        .geometry
                        .polygons
                        .get_mut(&last_pols_kind)
                        .ok_or(ParseError::MissingPolygonsList)?
                        .vertex_maps
                        .push(DiscontinousVertexMap::read_be_args(
                            &mut reader,
                            chunk_header.size,
                        )?);
                }
                "PTAG" => {
                    layers
                        .last_mut()
                        .ok_or(ParseError::MissingLayer)?
                        .geometry
                        .polygons
                        .get_mut(&last_pols_kind)
                        .ok_or(ParseError::MissingPolygonsList)?
                        .tags
                        .push(PolygonTagMapping::read_be_args(
                            &mut reader,
                            chunk_header.size,
                        )?);
                }
                "3GRP" => trisurfs.push(TriSurfGroupHeader::read_be(&mut reader)?),
                "3SRF" => trisurfs
                    .last_mut()
                    .ok_or(ParseError::MissingTriSurfGroupHeader)?
                    .trisurfaces
                    .push(TriSurfDataHeader::read_be(&mut reader)?),
                "VRTS" => {
                    let trisurf = Self::get_last_mut_trisurf(&mut trisurfs)?;

                    // Check the vertex count matches the expected size
                    if trisurf.vertex_count != chunk_header.size / 12 {
                        return Err(ParseError::InvalidSize);
                    }

                    trisurf.vertices = Some(TriSurfVertices::read_be_args(
                        &mut reader,
                        (trisurf.vertex_count,),
                    )?);
                }
                "TRIS" => {
                    let trisurf = Self::get_last_mut_trisurf(&mut trisurfs)?;

                    // Check the triangle count matches the expected size
                    if trisurf.triangle_count != chunk_header.size / 12 {
                        return Err(ParseError::InvalidSize);
                    }

                    trisurf.triangles = Some(TriSurfTriangles::read_be_args(
                        &mut reader,
                        (trisurf.triangle_count,),
                    )?);
                }
                "VVEC" => {
                    let trisurf = Self::get_last_mut_trisurf(&mut trisurfs)?;
                    trisurf.vectors.push(TriSurfVertexVectors::read_be_args(
                        &mut reader,
                        chunk_header.size,
                    )?);
                }
                "TTGS" => {
                    let trisurf = Self::get_last_mut_trisurf(&mut trisurfs)?;
                    trisurf.tags = Some(TriSurfTags::read_be_args(&mut reader, chunk_header.size)?);
                }
                "ITEM" => items.push(Item::read_be_args(&mut reader, chunk_header.size)?),
                "ENVL" => envelopes.push(Envelope::read_be(&mut reader)?),
                "ACTN" => actions.push(Action::read_be_args(&mut reader, chunk_header.size)?),
                "DATA" => {
                    data_blocks.push(DataBlock::read_be_args(&mut reader, chunk_header.size)?)
                }
                "AANI" => audio = Some(Audio::read_be_args(&mut reader, chunk_header.size)?),
                _ => {
                    // just eprint? or keep in vec<unknowns>?
                    eprintln!(
                        "Unknown Chunk {}, pos: {}, size: {}",
                        chunk_header.kind,
                        chunk_start_position,
                        chunk_header.size + 8 // size of header + data
                    );

                    reader.seek_relative(chunk_header.size as i64)?
                }
            }

            let consumed = reader.stream_position()? - chunk_start_position as u64 - 8;
            if consumed != chunk_header.size as u64 {
                return Err(ParseError::ChunkBoundaryMismatch {
                    kind: chunk_header.kind.to_string(),
                    expected: chunk_header.size,
                    consumed,
                });
            }
        }

        Ok(LuxologyFile {
            header,
            preview,
            description,
            version,
            application_version,
            encoding,
            included_subscene,
            subscenes,
            references,
            item_tags,
            channel_names,
            layers,
            trisurfs,
            items,
            envelopes,
            actions,
            data_blocks,
            audio,
        })
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
}
