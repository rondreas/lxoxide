use crate::item::SubChunkHeader;
use std::io::{Read, Seek};
use binrw::{BinRead, BinResult};

pub struct Audio {
    pub item: Option<u32>,  // assuming this is index to item list
    pub settings: Option<AudioSettings>
}

impl BinRead for Audio {
    type Args<'a> = u32;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        size: Self::Args<'_>,
    ) -> BinResult<Self> {
        let mut item = None;
        let mut settings = None;

        let start = reader.stream_position()?;

        while reader.stream_position()? - start < size as u64 {
            let header = SubChunkHeader::read_be(reader)?;
            match header.kind.as_str() {
                "AAIT" => { item = Some(u32::read_be(reader)?) },
                "AASE" => { settings = Some(AudioSettings::read_be(reader)?) },
                _ => {
                    // get the position for start of this subchunk,
                    let pos = reader.stream_position()? - 6;
                    eprintln!("Unhandled subchunk {} in AANI at {}", header.kind, pos);
                    // seek past it to skip.
                    reader.seek_relative(header.size as i64)?;
                }
            }
        }

        Ok(Audio{ item, settings })
    }
}

#[derive(BinRead, Debug)]
#[br(big)]
pub struct AudioSettings {
    pub r#loop: u16,
    pub mute: u16,
    pub scrub: u16,
    pub start: f32,
}
