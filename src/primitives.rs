use binrw::{BinRead, BinWrite, BinResult, NullString, Endian};
use binrw::meta::{ReadEndian, EndianKind};
use std::io::{Read, Write, Seek};

#[derive(Debug, Clone, PartialEq)]
pub struct S0(pub NullString);

#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct U2(pub u16);

#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq)]
#[br(big)]
#[bw(big)]
pub struct U4(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub enum VX {
    U2(u16),
    U4(u32)
}

impl BinRead for VX {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<Self> {
        let a = u16::read_be(reader)?;
        if a < 0xff00 {
            return Ok(VX::U2(a));
        }
        let b = u16::read_be(reader)?;
        Ok(VX::U4((((a as u32) << 16) | (b as u32)) & 0x00ff_ffff))
    }
}

impl ReadEndian for VX {
    const ENDIAN: EndianKind = EndianKind::Endian(binrw::Endian::Big);
}

impl BinRead for S0 {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<Self> {
        let s = NullString::read_options(reader, endian, ())?;
        if !reader.stream_position().unwrap().is_multiple_of(2) {
            reader.seek_relative(1)?
        }
        Ok(S0(s))
    }
}

impl BinWrite for S0 {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<()> {
        self.0.write_options(writer, endian, _args)?;
        if !writer.stream_position().unwrap().is_multiple_of(2) {
            writer.write_all(&[0x00])?;
        }
        Ok(())
    }
}

impl std::ops::Deref for S0 {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for S0 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&str> for S0 {
    fn from(s: &str) -> Self {
        Self(NullString(s.as_bytes().to_vec()))
    }
}

impl From<String> for S0 {
    fn from(s: String) -> Self {
        Self(NullString(s.into_bytes()))
    }
}

impl From<S0> for Vec<u8> {
    fn from(s: S0) -> Self {
        s.0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use binrw::BinRead;

    #[test]
    fn test_s0() {
        let mut reader = Cursor::new(b"a\0aa\0\0aaa\0");

        let s: S0 = S0::read_be(&mut reader).unwrap();
        assert_eq!(s, "a".into());

        let s: S0 = S0::read_be(&mut reader).unwrap();
        assert_eq!(s, "aa".into());

        let s: S0 = S0::read_be(&mut reader).unwrap();
        assert_eq!(s, "aaa".into());
    }

    #[test]
    fn test_vx() {
        let mut reader = Cursor::new(b"\xfe\xff\xff\xff");
        let vx = VX::read(&mut reader).unwrap();
        assert_eq!(vx, VX::U2(65_279));

        let mut reader = Cursor::new(b"\xff\x00\xff\x00");
        let vx = VX::read(&mut reader).unwrap();
        assert_eq!(vx, VX::U4(65_280));
    }
}
