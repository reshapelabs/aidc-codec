use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanInput<'a> {
    pub symbology_identifier: &'a str,
    pub raw: &'a [u8],
}

impl<'a> ScanInput<'a> {
    pub fn new(symbology_identifier: &'a str, raw: &'a [u8]) -> Self {
        Self {
            symbology_identifier,
            raw,
        }
    }

    pub fn from_aim_scan(scan: &'a [u8]) -> Result<Self, AidcError> {
        if scan.len() < 3 {
            return Err(AidcError::InvalidInput(
                "AIM scan must include a 3-byte symbology identifier".to_owned(),
            ));
        }
        if scan[0] != b']' {
            return Err(AidcError::InvalidInput(
                "AIM scan must start with ']'".to_owned(),
            ));
        }
        let symbology_identifier = std::str::from_utf8(&scan[..3])
            .map_err(|_| AidcError::InvalidInput("invalid UTF-8 in symbology identifier".to_owned()))?;
        Ok(Self {
            symbology_identifier,
            raw: &scan[3..],
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AidcError {
    #[error("unsupported symbology identifier: {0}")]
    UnsupportedSymbologyId(String),
    #[error("unsupported transport kind: {0}")]
    UnsupportedTransportKind(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
}

pub trait TransportDecoder {
    type Message;

    fn decode_transport(&self, input: ScanInput<'_>) -> Result<Self::Message, AidcError>;
}

pub trait PayloadParser<M> {
    type Output;

    fn parse_payload(&self, message: M) -> Result<Self::Output, AidcError>;
}

pub trait Codec {
    type Output;

    fn decode(&self, input: ScanInput<'_>) -> Result<Self::Output, AidcError>;
}
