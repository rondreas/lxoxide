use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek};
use std::path::Path as StdPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ID4(u32);

impl ID4 {
    pub const fn from_str(s: &str) -> Self {
        let b = s.as_bytes();

        assert!(b.len() == 4, "ID4 must be 4 bytes");
        assert!((b[0].is_ascii() && b[1].is_ascii() && b[2].is_ascii() && b[3].is_ascii()), "All bytes must be valid ascii");

        let b0 = b[0] as u32;
        let b1 = b[1] as u32;
        let b2 = b[2] as u32;
        let b3 = b[3] as u32;

        ID4(b0 << 24 | b1 << 16 | b2 << 8 | b3 )
    }

    pub const fn from_bytes(b: [u8; 4]) -> Self {
        assert!((b[0].is_ascii() && b[1].is_ascii() && b[2].is_ascii() && b[3].is_ascii()), "All bytes must be valid ascii");
        ID4((b[0] as u32) << 24 | (b[1] as u32) << 16 | (b[2] as u32) << 8 | b[3] as u32)
    }

    pub const fn to_bytes(self) -> [u8; 4] {
        let val = self.0;
        [
            (val >> 24) as u8,
            (val >> 16) as u8,
            (val >> 8) as u8,
            val as u8,
        ]
    }
}

impl fmt::Display for ID4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.to_bytes();
        write!(f, "{}{}{}{}", b[0] as char, b[1] as char, b[2] as char, b[3] as char)
    }
}


const FORM: ID4 = ID4::from_str("FORM");


#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extension {
    LXOB = 0x4c584f42,  // scene file
    LXPR = 0x4c585052,  // preset assembly
    LXPE = 0x4c585045,  // preset environment
    LXPM = 0x4c58504d,  // preset item
}

impl Extension {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x4c584f42 => Some(Extension::LXOB),
            0x4c585052 => Some(Extension::LXPR),
            0x4c585045 => Some(Extension::LXPE),
            0x4c58504d => Some(Extension::LXPM),
            _ => None,
        }
    }
}


// IFF header, 12 bytes at the start of the file
pub struct Header {
    pub form: ID4,
    pub size: u32,
    pub kind: Extension,
}



// Chunks are the blocks of data contained in the file
pub struct Chunk {
    pub kind: ID4,
    pub size: u32,
    pub data: Vec<u8>,
}


#[derive(Debug)]
pub enum ParseError {
    InvalidMagicNumber,
    SizeMismatch,
    InvalidSize,
    NonSupportedExtension,
    BufferTooShort,
    MissingNullTerminator,
    UnalignedBytes,
    IoError(io::Error),
}

impl PartialEq for ParseError {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (ParseError::InvalidMagicNumber, ParseError::InvalidMagicNumber)
                | (ParseError::SizeMismatch, ParseError::SizeMismatch)
                | (ParseError::InvalidSize, ParseError::InvalidSize)
                | (ParseError::NonSupportedExtension, ParseError::NonSupportedExtension)
                | (ParseError::BufferTooShort, ParseError::BufferTooShort)
                | (ParseError::MissingNullTerminator, ParseError::MissingNullTerminator)
                | (ParseError::UnalignedBytes, ParseError::UnalignedBytes)
                | (ParseError::IoError(_), ParseError::IoError(_))
        )
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidMagicNumber => write!(f, "IFF files must start with FORM"),
            ParseError::SizeMismatch => write!(f, "File size does not match reported size in header"),
            ParseError::InvalidSize => write!(f, "Invalid size for fixed size chunk data"),
            ParseError::NonSupportedExtension => write!(f, "File type not supported"),
            ParseError::BufferTooShort => write!(f, "Buffer is too short for the data to be parsed"),
            ParseError::MissingNullTerminator => write!(f, "Strings must be null terminated"),
            ParseError::UnalignedBytes => write!(f, "Bytes must be aligned to even number"),
            ParseError::IoError(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::IoError(e) => Some(e),
            _ => None,
        }
    }
}


pub struct LuxologyFile {
    pub header: Header,
    pub chunks: Vec<Chunk>,
}

impl LuxologyFile {
    pub fn new(header: Header, chunks: Vec<Chunk>) -> LuxologyFile {
        LuxologyFile { header, chunks }
    }

    pub fn from_path<P: AsRef<StdPath>>(path: P) -> Result<LuxologyFile, ParseError> {
        let file = File::open(path).map_err(ParseError::IoError)?;
        let mut reader = BufReader::new(file);

        let header = Self::parse_header(&mut reader)?;
        let chunks = Self::parse_chunks(&mut reader, header.size)?;

        Ok(LuxologyFile::new(header, chunks))
    }

    fn parse_header<R: Read>(reader: &mut R) -> Result<Header, ParseError> {
        let mut buf = [0u8; 12];
        reader.read_exact(&mut buf).map_err(ParseError::IoError)?;

        let form = ID4::from_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if form != FORM {
            return Err(ParseError::InvalidMagicNumber);
        }

        let size = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);

        let kind_raw = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let kind = Extension::from_u32(kind_raw).ok_or(ParseError::NonSupportedExtension)?;

        Ok(Header { form, size, kind })
    }

    fn parse_chunks<R: Read + Seek>(
        reader: &mut R,
        form_size: u32,
    ) -> Result<Vec<Chunk>, ParseError> {
        let mut chunks = Vec::new();
        let mut position: u64 = 0;
        let form_size = form_size as u64;

        while position < form_size {
            let mut chunk_header = [0u8; 8];
            match reader.read_exact(&mut chunk_header) {
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(ParseError::IoError(e)),
            }

            let kind = ID4::from_bytes([
                chunk_header[0],
                chunk_header[1],
                chunk_header[2],
                chunk_header[3],
            ]);
            let chunk_size =
                u32::from_be_bytes([chunk_header[4], chunk_header[5], chunk_header[6], chunk_header[7]]);

            let mut data = vec![0u8; chunk_size as usize];
            if chunk_size > 0 {
                reader.read_exact(&mut data).map_err(ParseError::IoError)?;
            }

            // IFF spec: odd-sized chunks are padded with a single byte
            if chunk_size % 2 != 0 {
                let mut padding = [0u8; 1];
                if let Err(e) = reader.read_exact(&mut padding) {
                    if e.kind() != io::ErrorKind::UnexpectedEof {
                        return Err(ParseError::IoError(e));
                    }
                }
            }

            position += (chunk_size as u64) + 8;

            chunks.push(Chunk {
                kind,
                size: chunk_size,
                data,
            });
        }

        Ok(chunks)
    }
}


#[derive(Debug, PartialEq)]
pub struct Version {
    major: u32,
    minor: u32,
    application: Vec<u8>
}

impl TryFrom<Vec<u8>> for Version {
    type Error = ParseError;
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        if vec.len() < 10 {
            return Err(Self::Error::BufferTooShort);
        }

        if vec.len() % 2 != 0 {
            return Err(Self::Error::UnalignedBytes);
        }

        let major = u32::from_be_bytes(vec[0..4].try_into().unwrap());
        let minor = u32::from_be_bytes(vec[4..8].try_into().unwrap());
        let application = &vec[8..];

        Ok(Version{major, minor, application: application.to_vec()})
    }
}

// The application version matches up with the nexus2000.dll which is used in Modo 16
#[derive(Debug, PartialEq)]
pub struct ApplicationVersion {
    base: u32,
    major: u32,
    minor: u32,
    build: u32,
    application: Vec<u8>
}

impl TryFrom<Vec<u8>> for ApplicationVersion {
    type Error = ParseError;
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        // The APPV chunk has to contain AT LEAST 18 bytes...
        if vec.len() < 18 {
            return Err(Self::Error::BufferTooShort);
        }

        // As always, it has to be aligend to even number of bytes...
        if vec.len() % 2 != 0 {
            return Err(Self::Error::UnalignedBytes);
        }

        let base = u32::from_be_bytes(vec[0..4].try_into().unwrap());
        let major = u32::from_be_bytes(vec[4..8].try_into().unwrap());
        let minor = u32::from_be_bytes(vec[8..12].try_into().unwrap());
        let build = u32::from_be_bytes(vec[12..16].try_into().unwrap());
        let application = &vec[16..];

        Ok(ApplicationVersion { base, major, minor, build, application: application.to_vec() })
    }
}

#[derive(Debug, PartialEq)]
#[repr(u32)]
pub enum Encoding {
    Default,
    Ansi,
    Utf8,
    ShiftJis,
    EucJp,
    EucKr,
    Gb2312,
    Big5
}

impl TryFrom<Vec<u8>> for Encoding {
    type Error = ParseError;
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        if vec.len() != 4 {
            return Err(Self::Error::InvalidSize);
        }

        let value = u32::from_be_bytes([vec[0], vec[1], vec[2], vec[3]]);

        match value {
            0 => Ok(Encoding::Default),
            1 => Ok(Encoding::Ansi),
            2 => Ok(Encoding::Utf8),
            3 => Ok(Encoding::ShiftJis),
            4 => Ok(Encoding::EucJp),
            5 => Ok(Encoding::EucKr),
            6 => Ok(Encoding::Gb2312),
            7 => Ok(Encoding::Big5),
            _ => Err(Self::Error::InvalidSize),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_chunk() {
        let data: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x10,  // major = 16
            0x00, 0x00, 0x00, 0x00,  // minor = 0
            0x6e, 0x65, 0x78, 0x75, 0x73, 0x20, 0x32, 0x30,
            0x30, 0x30, 0x20, 0x62, 0x79, 0x20, 0x54, 0x68,
            0x65, 0x20, 0x46, 0x6f, 0x75, 0x6e, 0x64, 0x72,
            0x79, 0x00,               // "nexus 2000 by The Foundry\0"
        ];

        let expected = Version{major: 16, minor: 0, application: b"nexus 2000 by The Foundry\0".to_vec()};
        let result = Version::try_from(data);

        assert_eq!(Ok(expected), result);
    }

    #[test]
    fn parse_application_version_chunk() {
        let data: Vec<u8> = vec![
            0x00, 0x00, 0x07, 0xd0,  // base = 2000
            0x00, 0x00, 0x07, 0xd0,  // major = 2000
            0x00, 0x00, 0x00, 0x00,  // minor = 0
            0x00, 0x0a, 0x17, 0xc6,  // build = 663110
            0x4d, 0x6f, 0x64, 0x6f, 0x20, 0x31, 0x36, 0x2e,
            0x30, 0x76, 0x31, 0x00,     // "Modo 16.0v1\0"
        ];

        let expected = ApplicationVersion {
            base: 2000,
            major: 2000,
            minor: 0,
            build: 661446,
            application: b"Modo 16.0v1\0".to_vec(),
        };

        let result = ApplicationVersion::try_from(data);
        assert_eq!(Ok(expected), result);
    }

    #[test]
    fn parse_encoding_chunk() {
        let data: Vec<u8> = vec![0x00, 0x00, 0x00, 0x02];
        let encoding = Encoding::try_from(data);
        assert_eq!(encoding, Ok(Encoding::Utf8));
    }

    #[test]
    fn id4_display() {
        let id = ID4::from_str("TEST");
        assert_eq!(format!("{}", id), "TEST");
        
        let id2 = ID4::from_bytes([b'L', b'X', b'O', b'B']);
        assert_eq!(format!("{}", id2), "LXOB");
    }

    #[test]
    fn id4_to_bytes() {
        let id = ID4::from_str("TEST");
        assert_eq!(id.to_bytes(), [b'T', b'E', b'S', b'T']);
        
        let id2 = ID4::from_bytes([b'L', b'X', b'O', b'B']);
        assert_eq!(id2.to_bytes(), [b'L', b'X', b'O', b'B']);
    }

    #[test]
    #[should_panic(expected = "All bytes must be valid ascii")]
    fn id4_from_bytes_non_ascii() {
        ID4::from_bytes([0xFF, 0xFF, 0xFF, 0xFF]);
    }
}
