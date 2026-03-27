use aidc_core::AidcError;

use crate::model::{AiElement, ParseResult, ParsedPayload, Transport};

pub fn parse_element_string(transport: Transport, normalized: Vec<u8>) -> Result<ParseResult, AidcError> {
    let elements = parse_ai_elements(&normalized);

    Ok(ParseResult {
        transport,
        parsed: ParsedPayload::Gs1ElementString {
            original: normalized,
            elements,
        },
    })
}

#[cfg(feature = "gs1-dl")]
pub fn parse_digital_link(transport: Transport, normalized: Vec<u8>) -> Result<ParseResult, AidcError> {
    let uri = String::from_utf8(normalized)
        .map_err(|_| AidcError::InvalidPayload("GS1 Digital Link must be valid UTF-8".to_owned()))?;

    Ok(ParseResult {
        transport,
        parsed: ParsedPayload::Gs1DigitalLink {
            uri,
            elements: Vec::new(),
        },
    })
}

#[cfg(feature = "gs1-composite")]
pub fn parse_composite_packet(
    transport: Transport,
    normalized: Vec<u8>,
) -> Result<ParseResult, AidcError> {
    Ok(ParseResult {
        transport,
        parsed: ParsedPayload::CompositePacket(normalized),
    })
}

fn parse_ai_elements(input: &[u8]) -> Vec<AiElement> {
    let text = String::from_utf8_lossy(input);

    vec![AiElement {
        ai: "raw".to_owned(),
        value: text.into_owned(),
    }]
}
