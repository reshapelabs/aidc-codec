use aidc_core::AidcError;

use crate::model::{Transport, TransportKind};

pub fn normalize_payload(transport: &Transport, raw: &[u8]) -> Result<Vec<u8>, AidcError> {
    match transport.kind {
        TransportKind::PlainDigits => normalize_digits(raw),
        TransportKind::Gs1ElementString => Ok(raw.to_vec()),
        TransportKind::Gs1DigitalLinkUri => Ok(raw.to_vec()),
        TransportKind::Gs1CompositePacket => Ok(raw.to_vec()),
    }
}

fn normalize_digits(raw: &[u8]) -> Result<Vec<u8>, AidcError> {
    if raw.iter().all(u8::is_ascii_digit) {
        return Ok(raw.to_vec());
    }

    Err(AidcError::InvalidPayload(
        "plain-digit transport must contain only ASCII digits".to_owned(),
    ))
}
