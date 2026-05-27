use crate::utils::read_aligned_nullstring;
use binrw::meta::{EndianKind, ReadEndian};
use binrw::{BinRead, BinResult, BinWrite, Endian, NullString};
use std::fmt;
use std::io::{Read, Seek, Write};
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

impl BinWrite for VX {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endian: Endian,
        (): Self::Args<'_>,
    ) -> BinResult<()> {
        match self {
            Self::U2(vx) => Ok((*vx).write_be(writer)?),
            Self::U4(vx) => Ok((0xff00_0000 | (0x00ff_ffff & *vx)).write_be(writer)?),
        }
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

#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
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
    // Legacy: Only found on CHNL and CHNV subchunks of ITEM
    Data(Vec<u8>),
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
        // 0x20 is Undefined Action, ie this channel is not animated? Or at least not present in
        // actions
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
    use std::io::Cursor;

    #[test]
    fn non_printable_ascii_id4_errors() {
        let mut reader = Cursor::new([35u8, 47u8, 201u8, 7u8]);
        let result = ID4::read_be(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn variable_index() {
        let mut reader = Cursor::new([0xfe, 0xff, 0xff, 0x00, 0xff, 0x00]);

        let small_index = VX::read_be(&mut reader).unwrap();
        let big_index = VX::read_be(&mut reader).unwrap();

        assert_eq!(small_index, VX::U2(65_279));
        assert_eq!(big_index, VX::U4(65_280));

        let mut writer = Cursor::new(vec![]);

        small_index.write_be(&mut writer).unwrap();
        big_index.write_be(&mut writer).unwrap();

        assert_eq!(writer.into_inner(), reader.into_inner());
    }
}
