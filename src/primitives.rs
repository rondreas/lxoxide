use crate::utils::read_aligned_nullstring;
use binrw::meta::{EndianKind, ReadEndian};
use binrw::{BinRead, BinWrite, BinResult, Endian, NullString};
use std::fmt;
use std::io::{Read, Write, Seek};
use std::str::FromStr;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ID4([u8; 4]);

impl ID4 {
    pub const fn new(val: [u8; 4]) -> Self {
        ID4(val)
    }

    pub fn from_bytes(b: [u8; 4]) -> Result<Self, crate::ParseError> {
        if !b.iter().all(|&x| (0x20..=0x7E).contains(&x)) {
            return Err(crate::ParseError::InvalidID4);
        }
        Ok(ID4(b))
    }

    pub const fn to_bytes(self) -> [u8; 4] {
        self.0
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("UNKN")
    }
}

impl BinRead for ID4 {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<Self> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        ID4::from_bytes(bytes).map_err(|e| binrw::Error::Custom {
            pos: reader.stream_position().unwrap_or(0),
            err: Box::new(e),
        })
    }
}

impl BinWrite for ID4 {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<()> {
        writer.write_all(&self.0).map_err(Into::into)
    }
}

impl PartialEq<&str> for ID4 {
    fn eq(&self, other: &&str) -> bool {
        other.len() == 4 && self.0 == other.as_bytes()
    }
}

impl PartialEq<ID4> for &str {
    fn eq(&self, other: &ID4) -> bool {
        self.len() == 4 && other.0 == self.as_bytes()
    }
}

impl From<ID4> for String {
    fn from(id4: ID4) -> String {
        String::from_utf8_lossy(&id4.0).to_string()
    }
}

impl FromStr for ID4 {
    type Err = crate::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();
        if b.len() != 4 {
            return Err(crate::ParseError::InvalidID4);
        }
        Ok(Self([b[0], b[1], b[2], b[3]]))
    }
}

impl fmt::Display for ID4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.to_bytes();
        write!(
            f,
            "{}{}{}{}",
            b[0] as char, b[1] as char, b[2] as char, b[3] as char
        )
    }
}

#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct ChunkHeader {
    pub kind: ID4,
    pub size: u32,
}

#[derive(BinRead, Debug)]
#[br(big)]
pub struct SubChunkHeader {
    pub kind: ID4,
    pub size: u16,
}

#[derive(BinRead, Debug, PartialEq)]
#[br(big)]
pub struct Point(pub [f32; 3]);

impl Point {
    pub fn x(&self) -> f32 {
        self.0[0]
    }

    pub fn y(&self) -> f32 {
        self.0[1]
    }

    pub fn z(&self) -> f32 {
        self.0[2]
    }
}

impl From<[f32; 3]> for Point {
    fn from(arr: [f32; 3]) -> Point {
        Point(arr)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelValue {
    Integer(i32),
    Float(f32),
    String(NullString),
}

impl ReadEndian for ChannelValue {
    const ENDIAN: EndianKind = EndianKind::Endian(Endian::Big);
}

impl BinRead for ChannelValue {
    type Args<'a> = (u16,);

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        flag: Self::Args<'_>,
    ) -> BinResult<Self> {
        match flag.0 & !0x20 {
            0x1 | 0x11 => Ok(ChannelValue::Integer(i32::read_be(reader)?)),
            0x2 | 0x12 => Ok(ChannelValue::Float(f32::read_be(reader)?)),
            0x3 | 0x13 => Ok(ChannelValue::String(read_aligned_nullstring(reader)?)),
            _ => {
                let pos = reader.stream_position()?;
                Err(binrw::Error::Custom {
                    pos,
                    err: Box::new(std::io::Error::other(format!(
                        "Invalid ItemFlag {} at: {}",
                        flag.0, pos
                    ))),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::BinRead;
    use std::io::Cursor;

    #[test]
    fn non_printable_ascii_id4_errors() {
        let mut reader = Cursor::new([35u8, 47u8, 201u8, 7u8]);
        let result = ID4::read_be(&mut reader);
        assert!(result.is_err());
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
