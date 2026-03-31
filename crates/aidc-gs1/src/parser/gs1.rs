use aidc_core::AidcError;

use crate::ai::{
    ai_requires_fnc1, fixed_value_length, is_known_ai, validate_ai_value, validate_message_rules,
};
#[cfg(feature = "gs1-dl")]
use crate::conformance::{parse_dl_uri, DlParseOptions};
use crate::model::{AiElement, Gs1Ai, ParseResult, ParsedPayload, Transport};

pub fn parse_element_string(
    transport: Transport,
    normalized: Vec<u8>,
) -> Result<ParseResult, AidcError> {
    let elements = parse_ai_elements(&normalized)?;
    validate_message_rules(elements.iter().map(|e| e.ai.code()))?;

    Ok(ParseResult {
        transport,
        parsed: ParsedPayload::Gs1ElementString {
            original: normalized,
            elements,
        },
    })
}

#[cfg(feature = "gs1-dl")]
pub fn parse_digital_link(
    transport: Transport,
    normalized: Vec<u8>,
) -> Result<ParseResult, AidcError> {
    let uri = String::from_utf8(normalized).map_err(|_| {
        AidcError::InvalidPayload("GS1 Digital Link must be valid UTF-8".to_owned())
    })?;
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
    validate_message_rules(elements.iter().map(|e| e.ai.code()))?;

    Ok(ParseResult {
        transport,
        parsed: ParsedPayload::Gs1DigitalLink { uri, elements },
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
    let text = std::str::from_utf8(input).map_err(|_| {
        AidcError::InvalidPayload("GS1 element string must be valid UTF-8".to_owned())
    })?;
    if text.is_empty() {
        return Err(AidcError::InvalidPayload(
            "empty GS1 element string".to_owned(),
        ));
    }

    let mut out = Vec::new();
    let fields = text.split('\u{001d}').collect::<Vec<_>>();
    for (idx, field) in fields.iter().enumerate() {
        if field.is_empty() {
            if idx + 1 == fields.len() && idx > 0 {
                continue;
            }
            return Err(AidcError::InvalidPayload(
                "empty FNC1-delimited field".to_owned(),
            ));
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
                return Err(AidcError::InvalidPayload(
                    "truncated fixed-length AI value".to_owned(),
                ));
            }
            let value = &body[..n];
            validate_ai_value(&ai, value)?;
            out.push(AiElement {
                ai: Gs1Ai::parse(&ai),
                value: value.to_owned(),
            });
            field = &body[n..];
            if !field.is_empty() && ai_requires_fnc1(&ai) {
                return Err(AidcError::InvalidPayload(
                    "FNC1 separator required after AI".to_owned(),
                ));
            }
            continue;
        }

        if body.is_empty() {
            return Err(AidcError::InvalidPayload(
                "empty variable-length AI value".to_owned(),
            ));
        }
        validate_ai_value(&ai, body)?;
        out.push(AiElement {
            ai: Gs1Ai::parse(&ai),
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
            return Err(AidcError::InvalidPayload(
                "empty internal AI segment".to_owned(),
            ));
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
    use proptest::prelude::*;

    use super::parse_ai_elements;
    use crate::ai::{is_known_ai, AI_DICTIONARY};
    use crate::model::{CarrierFamily, ParsedPayload, SymbologyId, Transport, TransportKind};

    #[test]
    fn parses_fixed_ai_element_string() {
        let elements = parse_ai_elements(b"0109520123456788").expect("parse should succeed");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].ai.code(), "01");
        assert_eq!(elements[0].value, "09520123456788");
    }

    #[test]
    fn parses_variable_ai_with_fnc1_separator() {
        let elements = parse_ai_elements(b"010952012345678810ABC123\x1d17251231")
            .expect("parse should succeed");
        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0].ai.code(), "01");
        assert_eq!(elements[0].value, "09520123456788");
        assert_eq!(elements[1].ai.code(), "10");
        assert_eq!(elements[1].value, "ABC123");
        assert_eq!(elements[2].ai.code(), "17");
        assert_eq!(elements[2].value, "251231");
    }

    #[test]
    fn allows_single_trailing_fnc1_separator() {
        let elements = parse_ai_elements(b"0109520123456788\x1d").expect("parse should succeed");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].ai.code(), "01");
    }

    #[test]
    fn predefined_fixed_ai_can_chain_without_separator() {
        let elements = parse_ai_elements(b"1701010110BATCH42").expect("parse should succeed");
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].ai.code(), "17");
        assert_eq!(elements[0].value, "010101");
        assert_eq!(elements[1].ai.code(), "10");
        assert_eq!(elements[1].value, "BATCH42");
    }

    #[test]
    fn non_predefined_fixed_ai_requires_separator_when_followed_by_more_data() {
        let err = parse_ai_elements(b"42620817010101").expect_err("parse should fail");
        assert!(err.to_string().contains("FNC1 separator required after AI"));
    }

    #[test]
    fn policy_variable_then_fixed_without_separator_is_single_variable_element() {
        let elements = parse_ai_elements(b"10BATCH4217010101").expect("parse should succeed");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].ai.code(), "10");
        assert_eq!(elements[0].value, "BATCH4217010101");
    }

    #[test]
    fn clause_mixed_predefined_and_variable_ordering_cases() {
        struct Case<'a> {
            name: &'a str,
            raw: &'a [u8],
            expected_elements: usize,
            expected_err: Option<&'a str>,
        }

        let cases = [
            Case {
                name: "predefined_then_variable_without_separator",
                raw: b"1701010110BATCH42",
                expected_elements: 2,
                expected_err: None,
            },
            Case {
                name: "predefined_then_variable_with_separator",
                raw: b"17010101\x1d10BATCH42",
                expected_elements: 2,
                expected_err: None,
            },
            Case {
                name: "variable_then_predefined_without_separator_is_ambiguous",
                raw: b"10BATCH4217010101",
                expected_elements: 1,
                expected_err: None,
            },
            Case {
                name: "variable_then_predefined_with_separator",
                raw: b"10BATCH42\x1d17010101",
                expected_elements: 2,
                expected_err: None,
            },
            Case {
                name: "single_trailing_separator_tolerated",
                raw: b"1701010110BATCH42\x1d",
                expected_elements: 2,
                expected_err: None,
            },
            Case {
                name: "double_separator_rejected",
                raw: b"17010101\x1d\x1d10BATCH42",
                expected_elements: 0,
                expected_err: Some("empty FNC1-delimited field"),
            },
        ];

        for case in cases {
            let got = parse_ai_elements(case.raw);
            if let Some(err_text) = case.expected_err {
                let err = got.unwrap_err();
                assert!(
                    err.to_string().contains(err_text),
                    "{} should include error {:?}, got {}",
                    case.name,
                    err_text,
                    err
                );
            } else {
                let parsed =
                    got.unwrap_or_else(|e| panic!("{} should parse, got {e}", case.name));
                assert_eq!(
                    parsed.len(),
                    case.expected_elements,
                    "{} expected {} elements",
                    case.name,
                    case.expected_elements
                );
            }
        }
    }

    #[test]
    fn rejects_truncated_fixed_value() {
        let err = parse_ai_elements(b"01095201234567").expect_err("parse should fail");
        assert!(err.to_string().contains("truncated fixed-length AI value"));
    }

    #[test]
    fn rejects_unknown_ai() {
        let err = parse_ai_elements(b"0409520123456788").expect_err("parse should fail");
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
        assert_eq!(
            parsed.to_hri().as_deref(),
            Some("(01)09520123456788(10)ABC123")
        );
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
                assert_eq!(elements[0].ai.code(), "01");
                assert_eq!(elements[0].value, "09520123456788");
                assert_eq!(elements[1].ai.code(), "10");
                assert_eq!(elements[1].value, "ABC123");
            }
            other => panic!("unexpected parsed payload: {other:?}"),
        }
    }

    fn fixed_ai_strategy() -> impl Strategy<Value = (String, String)> {
        prop_oneof![
            proptest::string::string_regex("[0-9]{17}")
                .expect("valid regex")
                .prop_map(|v| ("00".to_owned(), with_mod10_check_digit(&v))),
            proptest::string::string_regex("[0-9]{13}")
                .expect("valid regex")
                .prop_map(|v| ("01".to_owned(), with_mod10_check_digit(&v))),
            proptest::string::string_regex("[0-9]{12}")
                .expect("valid regex")
                .prop_map(|v| ("414".to_owned(), with_mod10_check_digit(&v))),
        ]
    }

    fn variable_ai_strategy() -> impl Strategy<Value = (String, String)> {
        prop_oneof![proptest::string::string_regex("[A-Z0-9]{1,20}")
            .expect("valid regex")
            .prop_map(|v| ("91".to_owned(), v)),]
    }

    fn ai_segment_strategy() -> impl Strategy<Value = (String, String)> {
        prop_oneof![fixed_ai_strategy(), variable_ai_strategy()]
    }

    fn with_mod10_check_digit(base: &str) -> String {
        let mut sum = 0u32;
        for (idx, ch) in base.chars().rev().enumerate() {
            let d = u32::from((ch as u8) - b'0');
            sum += if idx % 2 == 0 { 3 * d } else { d };
        }
        let check = (10 - (sum % 10)) % 10;
        format!("{base}{check}")
    }

    proptest! {
        #[test]
        fn hri_is_deterministic_for_parsed_element_strings(
            segments in prop::collection::vec(ai_segment_strategy(), 1..8)
        ) {
            let raw = segments
                .iter()
                .map(|(ai, value)| format!("{ai}{value}"))
                .collect::<Vec<_>>()
                .join("\u{001d}");
            let parsed = parse_ai_elements(raw.as_bytes()).expect("generated segment list must parse");
            let payload = ParsedPayload::Gs1ElementString {
                original: raw.into_bytes(),
                elements: parsed,
            };
            let hri1 = payload.to_hri().expect("HRI should exist");
            let hri2 = payload.to_hri().expect("HRI should exist");
            prop_assert_eq!(hri1, hri2);
        }

        #[test]
        fn fnc1_double_separator_is_rejected(
            segments in prop::collection::vec(ai_segment_strategy(), 2..8),
            insert_at in 0usize..7usize
        ) {
            let mut fields = segments
                .iter()
                .map(|(ai, value)| format!("{ai}{value}"))
                .collect::<Vec<_>>();
            let slot = insert_at % (fields.len() - 1);
            fields.insert(slot + 1, String::new());
            let raw = fields.join("\u{001d}");
            let err = parse_ai_elements(raw.as_bytes()).expect_err("empty FNC1 segment must fail");
            prop_assert!(err.to_string().contains("empty FNC1-delimited field"));
        }

        #[test]
        fn typed_ai_unknown_numeric_codes_remain_unknown(code in "[0-9]{2,4}") {
            prop_assume!(!is_known_ai(&code));
            let ai = crate::model::Gs1Ai::parse(&code);
            prop_assert_eq!(ai.code(), code.as_str());
            prop_assert!(ai.known().is_none());
        }

        #[test]
        fn typed_ai_dictionary_codes_are_known(index in 0usize..2500usize) {
            let keys = AI_DICTIONARY.entries().map(|(k, _)| *k).collect::<Vec<_>>();
            let code = keys[index % keys.len()];
            let ai = crate::model::Gs1Ai::parse(code);
            prop_assert_eq!(ai.code(), code);
            prop_assert!(ai.known().is_some());
        }
    }
}
