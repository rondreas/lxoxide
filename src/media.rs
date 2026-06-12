use crate::primitives::{ID4, SubChunkHeader};
use crate::utils::write_subchunk;
use binrw::{BinRead, BinResult, BinWrite, Endian};
use std::io::{Read, Seek, Write};
use std::str::FromStr;

// TODO: Look into making a lxo file with multiple audio files - or however one work with audio
// in Modo.

#[derive(Debug)]
pub struct Audio {
    pub item: Option<u32>, // assuming this is index to item list
    pub settings: Option<AudioSettings>,
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
                "AAIT" => item = Some(u32::read_be(reader)?),
                "AASE" => settings = Some(AudioSettings::read_be(reader)?),
                _ => {
                    // get the position for start of this subchunk,
                    let pos = reader.stream_position()? - 6;
                    eprintln!("Unhandled subchunk {} in AANI at {}", header.kind, pos);
                    // seek past it to skip.
                    reader.seek_relative(header.size as i64)?;
                }
            }
        }

        Ok(Audio { item, settings })
    }
}

impl BinWrite for Audio {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<()> {
        if let Some(item) = &self.item {
            write_subchunk(writer, ID4::from_str("AAIT").unwrap(), item)?;
        }

        if let Some(settings) = &self.settings {
            write_subchunk(writer, ID4::from_str("AASE").unwrap(), settings)?;
        }

        Ok(())
    }
}

#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct AudioSettings {
    pub r#loop: u16,
    pub mute: u16,
    pub scrub: u16,
    pub start: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{ChunkHeader, ID4};
    use crate::utils::write_chunk;
    use std::io::Cursor;

    #[test]
    fn default_audio() {
        let mut reader = Cursor::new([
            0x41, 0x41, 0x4e, 0x49, 0x00, 0x00, 0x00, 0x10, 0x41, 0x41, 0x53, 0x45, 0x00, 0x0a,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ]);

        let header = ChunkHeader::read_be(&mut reader).unwrap();
        let audio = Audio::read_be_args(&mut reader, header.size).unwrap();

        assert_eq!(audio.item, None);
        assert_eq!(
            audio.settings,
            Some(AudioSettings {
                r#loop: 0,
                mute: 0,
                scrub: 1,
                start: 0.0
            })
        );

        let mut writer = Cursor::new(vec![]);
        write_chunk(&mut writer, ID4::from_str("AANI").unwrap(), &audio).unwrap();
        assert_eq!(writer.into_inner(), reader.into_inner());
    }
}
