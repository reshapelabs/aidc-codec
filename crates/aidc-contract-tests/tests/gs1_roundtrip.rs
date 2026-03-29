use aidc_core::{AidcError, CanonicalPayload, DataElement, EncodeInput, ScanInput, TransportCodec};
use aidc_gs1::{Gs1Codec, ParseResult, ParsedPayload, SymbologyId, TransportKind};
use proptest::prelude::*;

fn elements_semantic(parsed: &ParsedPayload) -> Option<Vec<(String, String)>> {
    parsed.ai_elements().map(|elements| {
        elements
            .iter()
            .map(|e| (e.ai.code().to_owned(), e.value.clone()))
            .collect()
    })
}

fn to_encode_input_from_decoded(decoded: &ParseResult) -> Option<EncodeInput> {
    let sym = decoded.transport.symbology_id.as_str().to_owned();
    match &decoded.parsed {
        ParsedPayload::Digits(s) => Some(EncodeInput {
            symbology_identifier: sym,
            payload: CanonicalPayload::Digits(s.clone()),
        }),
        ParsedPayload::Gs1ElementString { elements, .. } => Some(EncodeInput {
            symbology_identifier: sym,
            payload: CanonicalPayload::Elements(
                elements
                    .iter()
                    .map(|e| DataElement {
                        id: e.ai.code().to_owned(),
                        value: e.value.clone(),
                    })
                    .collect(),
            ),
        }),
        ParsedPayload::Gs1DigitalLink { .. } | ParsedPayload::CompositePacket(_) => None,
    }
}

#[test]
fn encode_then_decode_preserves_gs1_semantics() {
    let codec = Gs1Codec;
    let req = EncodeInput {
        symbology_identifier: "]d2".to_owned(),
        payload: CanonicalPayload::Elements(vec![
            DataElement {
                id: "01".to_owned(),
                value: "09520123456788".to_owned(),
            },
            DataElement {
                id: "10".to_owned(),
                value: "ABC123".to_owned(),
            },
            DataElement {
                id: "17".to_owned(),
                value: "251231".to_owned(),
            },
        ]),
    };
    let encoded = codec.encode(req).expect("encode must succeed");
    let decoded = codec
        .decode(ScanInput::new(&encoded.symbology_identifier, &encoded.raw))
        .expect("decode must succeed");

    assert_eq!(decoded.transport.kind, TransportKind::Gs1ElementString);
    assert_eq!(decoded.transport.symbology_id, SymbologyId::D2);
    assert_eq!(
        decoded.to_hri().as_deref(),
        Some("(01)09520123456788(10)ABC123(17)251231")
    );
    assert_eq!(
        elements_semantic(&decoded.parsed),
        Some(vec![
            ("01".to_owned(), "09520123456788".to_owned()),
            ("10".to_owned(), "ABC123".to_owned()),
            ("17".to_owned(), "251231".to_owned()),
        ])
    );
}

#[test]
fn decode_encode_decode_is_semantically_stable_for_known_scan() {
    let codec = Gs1Codec;
    let first = codec
        .decode(ScanInput::new(
            "]d2",
            b"010952012345678810ABC123\x1d17251231",
        ))
        .expect("initial decode");
    let req = to_encode_input_from_decoded(&first).expect("encodable decoded payload");
    let encoded = codec.encode(req).expect("encode");
    let second = codec
        .decode(ScanInput::new(&encoded.symbology_identifier, &encoded.raw))
        .expect("second decode");

    assert_eq!(first.transport.kind, second.transport.kind);
    assert_eq!(first.transport.symbology_id, second.transport.symbology_id);
    assert_eq!(first.to_hri(), second.to_hri());
    assert_eq!(
        elements_semantic(&first.parsed),
        elements_semantic(&second.parsed)
    );
}

#[test]
fn encode_invalid_payload_returns_invalid_payload_error() {
    let codec = Gs1Codec;
    let err = codec
        .encode(EncodeInput {
            symbology_identifier: "]d2".to_owned(),
            payload: CanonicalPayload::Elements(vec![DataElement {
                id: "01".to_owned(),
                value: "ABC".to_owned(),
            }]),
        })
        .expect_err("encode should fail");
    assert!(matches!(err, AidcError::InvalidPayload(_)));
}

#[test]
fn decode_malformed_payload_returns_invalid_payload_error() {
    let codec = Gs1Codec;
    let err = codec
        .decode(ScanInput::new("]d2", b"0409520123456788"))
        .expect_err("decode should fail");
    assert!(matches!(err, AidcError::InvalidPayload(_)));
}

fn fixed_ai_strategy() -> impl Strategy<Value = DataElement> {
    prop_oneof![
        proptest::string::string_regex("[0-9]{17}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "00".to_owned(),
                value: with_mod10_check_digit(&v)
            }),
        proptest::string::string_regex("[0-9]{13}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "01".to_owned(),
                value: with_mod10_check_digit(&v)
            }),
        (0u8..100, 1u8..13, prop_oneof![Just(0u8), 1u8..29]).prop_map(|(yy, mm, dd)| DataElement {
            id: "17".to_owned(),
            value: format!("{yy:02}{mm:02}{dd:02}")
        }),
    ]
}

fn variable_ai_strategy() -> impl Strategy<Value = DataElement> {
    prop_oneof![
        proptest::string::string_regex("[A-Z0-9]{1,20}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "10".to_owned(),
                value: v
            }),
        proptest::string::string_regex("[A-Z0-9]{1,20}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "21".to_owned(),
                value: v
            }),
        proptest::string::string_regex("[A-Z0-9]{1,28}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "235".to_owned(),
                value: v
            }),
    ]
}

fn element_strategy() -> impl Strategy<Value = DataElement> {
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
    fn property_roundtrip_encode_decode_elements(elements in prop::collection::vec(element_strategy(), 1..8)) {
        let codec = Gs1Codec;
        let req = EncodeInput {
            symbology_identifier: "]d2".to_owned(),
            payload: CanonicalPayload::Elements(elements.clone()),
        };
        let encoded = codec.encode(req).expect("encode should succeed");
        let decoded = codec
            .decode(ScanInput::new(&encoded.symbology_identifier, &encoded.raw))
            .expect("decode should succeed");
        let got = elements_semantic(&decoded.parsed).expect("decoded should contain elements");
        let expected = elements
            .iter()
            .map(|e| (e.id.clone(), e.value.clone()))
            .collect::<Vec<_>>();
        prop_assert_eq!(got, expected);
        prop_assert!(decoded.to_hri().is_some());
    }
}
