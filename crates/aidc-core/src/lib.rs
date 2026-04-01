use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanInput<'a> {
    pub symbology_identifier: &'a str,
    pub raw: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataElement {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalPayload {
    Digits(String),
    Elements(Vec<DataElement>),
    Composite {
        linear: String,
        elements: Vec<DataElement>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeInput {
    pub symbology_identifier: String,
    pub payload: CanonicalPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedScan {
    pub symbology_identifier: String,
    pub raw: Vec<u8>,
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
        let symbology_identifier = std::str::from_utf8(&scan[..3]).map_err(|_| {
            AidcError::InvalidInput("invalid UTF-8 in symbology identifier".to_owned())
        })?;
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

pub trait TransportCodec {
    type TransportMsg;
    type Decoded;
    type EncodeRequest;

    fn decode_transport(&self, input: ScanInput<'_>) -> Result<Self::TransportMsg, AidcError>;
    fn parse_payload(&self, message: Self::TransportMsg) -> Result<Self::Decoded, AidcError>;

    fn format_payload(&self, request: Self::EncodeRequest)
        -> Result<Self::TransportMsg, AidcError>;
    fn encode_transport(&self, message: Self::TransportMsg) -> Result<EncodedScan, AidcError>;

    fn decode(&self, input: ScanInput<'_>) -> Result<Self::Decoded, AidcError> {
        let message = self.decode_transport(input)?;
        self.parse_payload(message)
    }

    fn encode(&self, request: Self::EncodeRequest) -> Result<EncodedScan, AidcError> {
        let message = self.format_payload(request)?;
        self.encode_transport(message)
    }
}
