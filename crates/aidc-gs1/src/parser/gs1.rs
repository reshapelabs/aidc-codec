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
    let text = std::str::from_utf8(&normalized).map_err(|_| {
        AidcError::InvalidPayload("GS1 composite packet must be valid UTF-8".to_owned())
    })?;
    let Some((linear, cc)) = text.split_once("|]e0") else {
        return Err(AidcError::InvalidPayload(
            "composite packet must contain '|]e0' separator".to_owned(),
        ));
    };
    if linear.is_empty() {
        return Err(AidcError::InvalidPayload(
            "composite packet linear component must not be empty".to_owned(),
        ));
    }
    if cc.is_empty() {
        return Err(AidcError::InvalidPayload(
            "composite packet CC component must not be empty".to_owned(),
        ));
    }
    if cc.contains("|]e0") {
        return Err(AidcError::InvalidPayload(
            "composite packet must contain only one '|]e0' separator".to_owned(),
        ));
    }
    let linear = linear.to_owned();
    let cc = cc.to_owned();
    let cc_elements = parse_ai_elements(cc.as_bytes())?;

    Ok(ParseResult {
        transport,
        parsed: ParsedPayload::CompositePacket {
            original: normalized,
            linear,
            cc_elements,
        },
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
    for field in &fields {
        if field.is_empty() {
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
        if has_ambiguous_following_element(&ai, body) {
            return Err(AidcError::InvalidPayload(
                "FNC1 separator required after AI".to_owned(),
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

fn has_ambiguous_following_element(ai: &str, body: &str) -> bool {
    for (idx, _) in body.char_indices().skip(1) {
        let prefix = &body[..idx];
        if validate_ai_value(ai, prefix).is_err() {
            continue;
        }

        let suffix = &body[idx..];
        if parse_ai(suffix).is_none() {
            continue;
        }

        let mut parsed_suffix = Vec::new();
        if parse_field(suffix, &mut parsed_suffix).is_ok() && !parsed_suffix.is_empty() {
            return true;
        }
    }
    false
}

pub(crate) fn parse_internal_ai_string(input: &str) -> Result<Vec<AiElement>, AidcError> {
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
    fn rejects_single_trailing_fnc1_separator() {
        let err = parse_ai_elements(b"0109520123456788\x1d").expect_err("parse should fail");
        assert!(err.to_string().contains("empty FNC1-delimited field"));
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
    fn rejects_variable_then_fixed_without_separator() {
        let err = parse_ai_elements(b"10BATCH4217010101").expect_err("parse should fail");
        assert!(err.to_string().contains("FNC1 separator required after AI"));
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
                name: "variable_then_predefined_without_separator_rejected",
                raw: b"10BATCH4217010101",
                expected_elements: 0,
                expected_err: Some("FNC1 separator required after AI"),
            },
            Case {
                name: "variable_then_predefined_with_separator",
                raw: b"10BATCH42\x1d17010101",
                expected_elements: 2,
                expected_err: None,
            },
            Case {
                name: "single_trailing_separator_rejected",
                raw: b"1701010110BATCH42\x1d",
                expected_elements: 0,
                expected_err: Some("empty FNC1-delimited field"),
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
                let parsed = got.unwrap_or_else(|e| panic!("{} should parse, got {e}", case.name));
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

    #[cfg(feature = "gs1-composite")]
    #[test]
    fn parses_composite_packet_with_valid_separator() {
        let transport = Transport {
            symbology_id: SymbologyId::LowerE1,
            carrier: CarrierFamily::Gs1Composite,
            kind: TransportKind::Gs1CompositePacket,
        };
        let parsed =
            super::parse_composite_packet(transport, b"2112345678900|]e00109520123456788".to_vec())
                .expect("parse should succeed");
        match parsed.parsed {
            ParsedPayload::CompositePacket {
                original,
                linear,
                cc_elements,
            } => {
                assert_eq!(original, b"2112345678900|]e00109520123456788");
                assert_eq!(linear, "2112345678900");
                assert_eq!(cc_elements.len(), 1);
                assert_eq!(cc_elements[0].ai.code(), "01");
                assert_eq!(cc_elements[0].value, "09520123456788");
            }
            other => panic!("unexpected parsed payload: {other:?}"),
        }
    }

    #[cfg(feature = "gs1-composite")]
    #[test]
    fn rejects_composite_packet_without_separator() {
        let transport = Transport {
            symbology_id: SymbologyId::LowerE1,
            carrier: CarrierFamily::Gs1Composite,
            kind: TransportKind::Gs1CompositePacket,
        };
        let err =
            super::parse_composite_packet(transport, b"21123456789000109520123456788".to_vec())
                .expect_err("parse should fail");
        assert!(err
            .to_string()
            .contains("composite packet must contain '|]e0' separator"));
    }

    #[cfg(feature = "gs1-composite")]
    #[test]
    fn rejects_composite_packet_with_empty_cc_component() {
        let transport = Transport {
            symbology_id: SymbologyId::LowerE1,
            carrier: CarrierFamily::Gs1Composite,
            kind: TransportKind::Gs1CompositePacket,
        };
        let err = super::parse_composite_packet(transport, b"2112345678900|]e0".to_vec())
            .expect_err("parse should fail");
        assert!(err
            .to_string()
            .contains("composite packet CC component must not be empty"));
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
        prop_oneof![proptest::string::string_regex("[A-Z]{1,20}")
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
