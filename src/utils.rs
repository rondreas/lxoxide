use binrw::{BinRead, NullString};
use std::io::{Read, Seek};

pub fn read_aligned_nullstring<R: Read + Seek>(reader: &mut R) -> Result<NullString, binrw::Error> {
    let s = NullString::read_be(reader)?;
    if !reader.stream_position()?.is_multiple_of(2) {
        reader.seek_relative(1)?;
    }
    Ok(s)
}
