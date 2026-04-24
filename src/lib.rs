use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidMagicNumber,
    SizeMismatch,
    NonSupportedExtension,
    BufferTooShort,
    MissingNullTerminator,
    UnalignedBytes,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidMagicNumber => write!(f, "IFF files must start with FORM"),
            ParseError::SizeMismatch => write!(f, "File size does not match reported size in header"),
            ParseError::NonSupportedExtension => write!(f, "File type not supported"),
            ParseError::BufferTooShort => write!(f, "Buffer is too short for the data to be parsed"),
            ParseError::MissingNullTerminator => write!(f, "Strings must be null terminated"),
            ParseError::UnalignedBytes => write!(f, "Bytes must be aligned to even number"),
        }
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

        let expected = Version{major: 16, minor: 0, application: b"nexus 2000 by The Foundry".to_vec()};
        let result = Version::try_from(data);

        assert_eq!(Ok(expected), result);
    }
}
