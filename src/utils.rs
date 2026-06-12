use crate::primitives::{ChunkHeader, ID4, SubChunkHeader};
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
    let mut data = buf.into_inner();
    
    if data.len() % 2 != 0 {
        data.push(0);
    }

    SubChunkHeader {
        kind,
        size: data.len() as u16,
    }
    .write_be(writer)?;
    writer.write_all(&data)?;
    Ok(())
}

pub fn write_chunk<W: Write + Seek, T: BinWrite>(
    writer: &mut W,
    kind: ID4,
    value: &T,
) -> BinResult<()>
where
    for<'a> T::Args<'a>: Default,
{
    let mut buf = Cursor::new(Vec::new());
    value.write_be(&mut buf)?;
    let mut data = buf.into_inner();
    
    if data.len() % 2 != 0 {
        data.push(0);
    }
    
    ChunkHeader {
        kind,
        size: data.len() as u32,
    }
    .write_be(writer)?;
    writer.write_all(&data)?;
    Ok(())
}

pub fn read_nullstring_from_bytes(buf: &[u8]) -> Result<(NullString, usize), binrw::Error> {
    let null_pos = buf.iter().position(|&b| b == 0).ok_or_else(|| {
        binrw::Error::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "NullString missing null terminator",
        ))
    })?;
    let s = NullString(buf[..null_pos].to_vec());
    let mut offset = null_pos + 1;
    if offset % 2 != 0 {
        offset += 1;
    }
    Ok((s, offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nullstring_from_slice() {
        let bytes = [0x00, 0x00, 0x61, 0x61, 0x00, 0x00, 0x61, 0x00];

        let mut nullstrings: Vec<NullString> = Vec::with_capacity(3);
        let mut offset = 0;
        while offset < 8 {
            let (s, o) = read_nullstring_from_bytes(&bytes[offset..]).unwrap();
            offset += o;
            nullstrings.push(s);
        }

        assert_eq!(
            nullstrings,
            vec![
                NullString::from(""),
                NullString::from("aa"),
                NullString::from("a")
            ]
        );
    }
}
