use binrw::meta::{EndianKind, ReadEndian};
use binrw::{BinRead, BinResult, Endian};
use std::io::{Read, Seek};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VX {
    U2(u16),
    U4(u32),
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

impl fmt::Display for VX {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::U2(n) => write!(f, "{n}"),
            Self::U4(n) => write!(f, "{n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::BinRead;
    use std::io::Cursor;

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
