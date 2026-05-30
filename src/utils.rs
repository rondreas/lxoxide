use crate::primitives::{ID4, SubChunkHeader};
use binrw::{BinRead, BinResult, BinWrite, NullString};
use std::io::{Cursor, Read, Seek, Write};

pub fn read_aligned_nullstring<R: Read + Seek>(reader: &mut R) -> Result<NullString, binrw::Error> {
    let s = NullString::read_be(reader)?;
    if !reader.stream_position()?.is_multiple_of(2) {
        reader.seek_relative(1)?;
    }
    Ok(s)
}

pub fn write_aligned_nullstring<W: Write + Seek>(writer: &mut W, s: &NullString) -> BinResult<()> {
    s.write_be(writer)?;
    if !writer.stream_position()?.is_multiple_of(2) {
        0u8.write_be(writer)?;
    }
    Ok(())
}

pub fn write_subchunk<W: Write + Seek, T: BinWrite>(
    writer: &mut W,
    kind: ID4,
    value: &T,
) -> BinResult<()>
where
    for<'a> T::Args<'a>: Default,
{
    let mut buf = Cursor::new(Vec::new());
    value.write_be(&mut buf)?;
    let data = buf.into_inner();
    SubChunkHeader {
        kind,
        size: data.len() as u16,
    }
    .write_be(writer)?;
    writer.write_all(&data)?;
    Ok(())
}
