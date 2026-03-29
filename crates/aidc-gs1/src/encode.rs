use aidc_core::{AidcError, CanonicalPayload, DataElement};

use crate::ai::{ai_requires_fnc1, validate_ai_value, validate_message_rules};
use crate::model::TransportKind;

pub fn encode_payload(
    kind: TransportKind,
    payload: CanonicalPayload,
) -> Result<Vec<u8>, AidcError> {
    match (kind, payload) {
        (TransportKind::PlainDigits, CanonicalPayload::Digits(s)) => {
            if !s.as_bytes().iter().all(u8::is_ascii_digit) {
                return Err(AidcError::InvalidPayload(
                    "plain-digit transport must contain only ASCII digits".to_owned(),
                ));
            }
            Ok(s.into_bytes())
        }
        (TransportKind::Gs1ElementString, CanonicalPayload::Elements(elements)) => {
            encode_element_string(elements)
        }
        (TransportKind::Gs1DigitalLinkUri, _) => Err(AidcError::UnsupportedTransportKind(
            "gs1 digital link encode not implemented".to_owned(),
        )),
        (TransportKind::Gs1CompositePacket, _) => Err(AidcError::UnsupportedTransportKind(
            "gs1 composite packet encode not implemented".to_owned(),
        )),
        (TransportKind::PlainDigits, _) => Err(AidcError::InvalidPayload(
            "digits transport requires CanonicalPayload::Digits".to_owned(),
        )),
        (TransportKind::Gs1ElementString, _) => Err(AidcError::InvalidPayload(
            "element-string transport requires CanonicalPayload::Elements".to_owned(),
        )),
    }
}

fn encode_element_string(elements: Vec<DataElement>) -> Result<Vec<u8>, AidcError> {
    if elements.is_empty() {
        return Err(AidcError::InvalidPayload(
            "element-string payload requires at least one element".to_owned(),
        ));
    }

    let mut out = Vec::<u8>::new();
    validate_message_rules(elements.iter().map(|e| e.id.as_str()))?;
    for (idx, element) in elements.iter().enumerate() {
        if element.id.is_empty() || !element.id.chars().all(|c| c.is_ascii_digit()) {
            return Err(AidcError::InvalidPayload(
                "element id must be a numeric AI code".to_owned(),
            ));
        }
        validate_ai_value(&element.id, &element.value)?;

        if idx > 0 && ai_requires_fnc1(&elements[idx - 1].id) {
            out.push(0x1D);
        }
        out.extend_from_slice(element.id.as_bytes());
        out.extend_from_slice(element.value.as_bytes());
    }
    Ok(out)
}
