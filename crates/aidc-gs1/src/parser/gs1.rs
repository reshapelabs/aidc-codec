use aidc_core::AidcError;

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

fn parse_ai(field: &str) -> Option<String> {
    for len in [4usize, 3, 2] {
        if field.len() < len {
            continue;
        }
        let ai = &field[..len];
        if !ai.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if is_supported_ai(ai) {
            return Some(ai.to_owned());
        }
    }
    None
}

fn is_supported_ai(ai: &str) -> bool {
    if ai.len() == 4 {
        if let Some(prefix) = ai.get(..3) {
            if matches!(prefix, "310" | "392") {
                return true;
            }
        }
    }
    matches!(
        ai,
        "00"
            | "01"
            | "02"
            | "10"
            | "11"
            | "12"
            | "13"
            | "15"
            | "16"
            | "17"
            | "20"
            | "21"
            | "22"
            | "37"
            | "89"
            | "98"
            | "99"
            | "235"
            | "253"
            | "254"
            | "255"
            | "401"
            | "402"
            | "414"
            | "417"
            | "8003"
            | "8004"
            | "8006"
            | "8010"
            | "8013"
            | "8017"
            | "8018"
            | "8019"
            | "8020"
    )
}

fn fixed_value_length(ai: &str) -> Option<usize> {
    let bytes = ai.as_bytes();
    if bytes.len() < 2 || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
        return None;
    }
    let prefix = usize::from(bytes[0] - b'0') * 10 + usize::from(bytes[1] - b'0');
    const FIXED_BY_PREFIX: &[u8] = &[
        18, 14, 14, 14, 16, 0, 0, 0, 0, 0, 0, 6, 6, 6, 6, 6, 6, 6, 6, 6, 2, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 6, 6, 6, 6, 6, 6, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let n = FIXED_BY_PREFIX[prefix];
    if n == 0 {
        None
    } else {
        Some(n as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_ai_elements;

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
}
