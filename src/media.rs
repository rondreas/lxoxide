use crate::primitives::{ID4, SubChunkHeader};
use crate::utils::write_subchunk;
use binrw::{BinRead, BinResult, BinWrite, Endian};
use std::io::{Read, Seek, Write};
use std::str::FromStr;

/// Audio (`AANI`)
///
/// Configuration for audio playback within the scene.
///
/// Modo does not embed audio files in the scene but links to them.
#[derive(Debug, PartialEq)]
pub struct Audio {
    /// Index of the audio item in the scene's item list used for Timeline playback.
    pub item: Option<u32>,
    /// Playback settings for the selected audio file.
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

/// Audio Settings (`AASE`)
///
/// Settings for audio playback synchronization and behavior. Subchunk of Audio.
#[derive(BinRead, BinWrite, Debug, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct AudioSettings {
    /// Whether the audio plays repeatedly if it is shorter than the scene's
    /// start and end times.
    pub r#loop: u16,
    /// Whether audio playback is suspended.
    pub mute: u16,
    /// Whether the audio plays back when manually scrubbing the Timeline.
    pub scrub: u16,
    /// The frame on the Timeline where the audio file begins to play back.
    pub start: f32,
}
