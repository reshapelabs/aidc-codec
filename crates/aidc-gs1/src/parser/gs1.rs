use aidc_core::AidcError;

use crate::ai::{fixed_value_length, is_known_ai};
#[cfg(feature = "gs1-dl")]
use crate::conformance::{parse_dl_uri, DlParseOptions};
use crate::model::{AiElement, ParseResult, ParsedPayload, Transport};

pub fn parse_element_string(transport: Transport, normalized: Vec<u8>) -> Result<ParseResult, AidcError> {
    let elements = parse_ai_elements(&normalized)?;

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
    let internal = parse_dl_uri(
        &uri,
        DlParseOptions {
            permit_zero_suppressed_gtin: true,
            validate_unknown_ai_not_dl_attr: true,
            ..DlParseOptions::default()
        },
    )
    .map_err(|e| AidcError::InvalidPayload(e.to_string()))?;
    let elements = parse_internal_ai_string(&internal)?;

    Ok(ParseResult {
        transport,
        parsed: ParsedPayload::Gs1DigitalLink {
            uri,
            elements,
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

fn parse_ai_elements(input: &[u8]) -> Result<Vec<AiElement>, AidcError> {
    let text = std::str::from_utf8(input)
        .map_err(|_| AidcError::InvalidPayload("GS1 element string must be valid UTF-8".to_owned()))?;
    if text.is_empty() {
        return Err(AidcError::InvalidPayload("empty GS1 element string".to_owned()));
    }

    let mut out = Vec::new();
    for field in text.split('\u{001d}') {
        if field.is_empty() {
            return Err(AidcError::InvalidPayload("empty FNC1-delimited field".to_owned()));
        }
        parse_field(field, &mut out)?;
    }
    Ok(out)
}

fn parse_field(mut field: &str, out: &mut Vec<AiElement>) -> Result<(), AidcError> {
    while !field.is_empty() {
        let ai = parse_ai(field)
            .ok_or_else(|| AidcError::InvalidPayload("unable to identify AI".to_owned()))?;
        let ai_len = ai.len();
        let body = &field[ai_len..];

        if let Some(n) = fixed_value_length(&ai) {
            if body.len() < n {
                return Err(AidcError::InvalidPayload("truncated fixed-length AI value".to_owned()));
            }
            let value = &body[..n];
            out.push(AiElement {
                ai,
                value: value.to_owned(),
            });
            field = &body[n..];
            continue;
        }

        if body.is_empty() {
            return Err(AidcError::InvalidPayload("empty variable-length AI value".to_owned()));
        }
        out.push(AiElement {
            ai,
            value: body.to_owned(),
        });
        field = "";
    }
    Ok(())
}

#[cfg(feature = "gs1-dl")]
fn parse_internal_ai_string(input: &str) -> Result<Vec<AiElement>, AidcError> {
    if !input.starts_with('^') || input.len() == 1 {
        return Err(AidcError::InvalidPayload(
            "internal GS1 AI string must start with '^' and contain data".to_owned(),
        ));
    }
    let mut out = Vec::new();
    for field in input[1..].split('^') {
        if field.is_empty() {
            return Err(AidcError::InvalidPayload("empty internal AI segment".to_owned()));
        }
        parse_field(field, &mut out)?;
    }
    Ok(out)
}

fn parse_ai(field: &str) -> Option<String> {
    for len in [4usize, 3, 2] {
        if field.len() < len {
            continue;
        }
        let ai = &field[..len];
        if !ai.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if is_known_ai(ai) {
            return Some(ai.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_ai_elements;
    use crate::model::{CarrierFamily, ParsedPayload, SymbologyId, Transport, TransportKind};

    #[test]
    fn parses_fixed_ai_element_string() {
        let elements = parse_ai_elements(b"0109520123456788").expect("parse should succeed");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].ai, "01");
        assert_eq!(elements[0].value, "09520123456788");
    }

    #[test]
    fn parses_variable_ai_with_fnc1_separator() {
        let elements = parse_ai_elements(b"010952012345678810ABC123\x1d17251231")
            .expect("parse should succeed");
        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0].ai, "01");
        assert_eq!(elements[0].value, "09520123456788");
        assert_eq!(elements[1].ai, "10");
        assert_eq!(elements[1].value, "ABC123");
        assert_eq!(elements[2].ai, "17");
        assert_eq!(elements[2].value, "251231");
    }

    #[test]
    fn rejects_truncated_fixed_value() {
        let err = parse_ai_elements(b"01095201234567").expect_err("parse should fail");
        assert!(err.to_string().contains("truncated fixed-length AI value"));
    }

    #[test]
    fn rejects_unknown_ai() {
        let err = parse_ai_elements(b"0309520123456788").expect_err("parse should fail");
        assert!(err.to_string().contains("unable to identify AI"));
    }

    #[test]
    fn parse_result_hri_formats_ai_elements() {
        let transport = Transport {
            symbology_id: SymbologyId::D2,
            carrier: CarrierFamily::Gs1DataMatrix,
            kind: TransportKind::Gs1ElementString,
        };
        let parsed = super::parse_element_string(transport, b"010952012345678810ABC123".to_vec())
            .expect("parse should succeed");
        assert_eq!(parsed.to_hri().as_deref(), Some("(01)09520123456788(10)ABC123"));
    }

    #[test]
    fn non_gs1_payload_has_no_hri() {
        let parsed = ParsedPayload::Digits("123".to_owned());
        assert_eq!(parsed.to_hri(), None);
    }

    #[cfg(feature = "gs1-dl")]
    #[test]
    fn parses_digital_link_into_ai_elements() {
        let transport = Transport {
            symbology_id: SymbologyId::Q1,
            carrier: CarrierFamily::Qr,
            kind: TransportKind::Gs1DigitalLinkUri,
        };
        let parsed = super::parse_digital_link(
            transport,
            b"https://id.gs1.org/01/09520123456788/10/ABC123".to_vec(),
        )
        .expect("parse should succeed");
        match parsed.parsed {
            ParsedPayload::Gs1DigitalLink { elements, .. } => {
                assert_eq!(elements.len(), 2);
                assert_eq!(elements[0].ai, "01");
                assert_eq!(elements[0].value, "09520123456788");
                assert_eq!(elements[1].ai, "10");
                assert_eq!(elements[1].value, "ABC123");
            }
            other => panic!("unexpected parsed payload: {other:?}"),
        }
    }
}
