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
        ParsedPayload::Gs1DigitalLink { .. } | ParsedPayload::CompositePacket { .. } => None,
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

#[test]
fn i1_decode_rejects_non_itf14_payload() {
    let codec = Gs1Codec;
    let err = codec
        .decode(ScanInput::new("]I1", b"9520123456788"))
        .expect_err("decode should fail");
    assert!(matches!(err, AidcError::InvalidPayload(_)));
}

#[test]
fn i1_encode_rejects_non_itf14_payload() {
    let codec = Gs1Codec;
    let err = codec
        .encode(EncodeInput {
            symbology_identifier: "]I1".to_owned(),
            payload: CanonicalPayload::Digits("9520123456788".to_owned()),
        })
        .expect_err("encode should fail");
    assert!(matches!(err, AidcError::InvalidPayload(_)));
}

#[test]
fn dl_encode_then_decode_preserves_gs1_semantics() {
    let codec = Gs1Codec;
    let req = EncodeInput {
        symbology_identifier: "]Q1".to_owned(),
        payload: CanonicalPayload::Elements(vec![
            DataElement {
                id: "01".to_owned(),
                value: "9520123456788".to_owned(),
            },
            DataElement {
                id: "10".to_owned(),
                value: "ABC123".to_owned(),
            },
            DataElement {
                id: "21".to_owned(),
                value: "SER42".to_owned(),
            },
            DataElement {
                id: "17".to_owned(),
                value: "201225".to_owned(),
            },
        ]),
    };
    let encoded = codec.encode(req).expect("encode must succeed");
    let uri = String::from_utf8(encoded.raw.clone()).expect("dl uri must be utf8");
    assert_eq!(
        uri,
        "https://id.gs1.org/01/09520123456788/10/ABC123/21/SER42?17=201225"
    );
    let decoded = codec
        .decode(ScanInput::new(&encoded.symbology_identifier, &encoded.raw))
        .expect("decode must succeed");
    assert_eq!(
        elements_semantic(&decoded.parsed),
        Some(vec![
            ("01".to_owned(), "09520123456788".to_owned()),
            ("10".to_owned(), "ABC123".to_owned()),
            ("21".to_owned(), "SER42".to_owned()),
            ("17".to_owned(), "201225".to_owned()),
        ])
    );
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
        proptest::string::string_regex("[A-Z]{1,20}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "10".to_owned(),
                value: v
            }),
        proptest::string::string_regex("[A-Z]{1,20}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "21".to_owned(),
                value: v
            }),
        proptest::string::string_regex("[A-Z]{1,28}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "235".to_owned(),
                value: v
            }),
        proptest::string::string_regex("[A-Z]{1,20}")
            .expect("valid regex")
            .prop_map(|v| DataElement {
                id: "91".to_owned(),
                value: v
            }),
    ]
}

fn dependent_element_strategy() -> impl Strategy<Value = DataElement> {
    prop_oneof![
        fixed_ai_strategy().prop_filter("exclude AI 01 from dependent list", |e| e.id != "01"),
        variable_ai_strategy()
    ]
}

fn gs1_element_carrier_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("]d2"),
        Just("]Q3"),
        Just("]C1"),
        Just("]e0"),
        Just("]J1"),
    ]
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
    fn property_roundtrip_encode_decode_elements(
        carrier in gs1_element_carrier_strategy(),
        extra in prop::collection::vec(dependent_element_strategy(), 0..7)
            .prop_filter("exclude known incompatible AI pairings", |v| {
                let has_21 = v.iter().any(|e| e.id == "21");
                let has_235 = v.iter().any(|e| e.id == "235");
                !(has_21 && has_235)
            })
    ) {
        let codec = Gs1Codec;
        let mut elements = vec![DataElement {
            id: "01".to_owned(),
            value: "09520123456788".to_owned(),
        }];
        elements.extend(extra.clone());
        let req = EncodeInput {
            symbology_identifier: carrier.to_owned(),
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

#[test]
fn encode_rejects_missing_required_association() {
    let codec = Gs1Codec;
    let err = codec
        .encode(EncodeInput {
            symbology_identifier: "]d2".to_owned(),
            payload: CanonicalPayload::Elements(vec![DataElement {
                id: "10".to_owned(),
                value: "LOT42".to_owned(),
            }]),
        })
        .expect_err("encode should fail");
    assert!(matches!(err, AidcError::InvalidPayload(_)));
}

#[test]
fn decode_rejects_missing_required_association() {
    let codec = Gs1Codec;
    let err = codec
        .decode(ScanInput::new("]d2", b"10LOT42"))
        .expect_err("decode should fail");
    assert!(matches!(err, AidcError::InvalidPayload(_)));
}

#[test]
fn q3_percent_and_gs_separator_are_semantically_equivalent() {
    let codec = Gs1Codec;
    let q3_percent = codec
        .decode(ScanInput::new("]Q3", b"0109520123456788%17251231"))
        .expect("decode should succeed");
    let q3_gs = codec
        .decode(ScanInput::new("]Q3", b"0109520123456788\x1d17251231"))
        .expect("decode should succeed");

    assert_eq!(
        elements_semantic(&q3_percent.parsed),
        elements_semantic(&q3_gs.parsed)
    );
    assert_eq!(q3_percent.to_hri(), q3_gs.to_hri());
}

#[test]
fn q3_percent_is_not_rewritten_when_gs_already_present() {
    let codec = Gs1Codec;
    let decoded = codec
        .decode(ScanInput::new(
            "]Q3",
            b"010952012345678810ABC%123\x1d17251231",
        ))
        .expect("decode should succeed");
    let got = elements_semantic(&decoded.parsed).expect("expected GS1 elements");
    assert_eq!(
        got,
        vec![
            ("01".to_owned(), "09520123456788".to_owned()),
            ("10".to_owned(), "ABC%123".to_owned()),
            ("17".to_owned(), "251231".to_owned()),
        ]
    );
}

#[test]
fn d2_gs_separator_decodes_to_expected_elements() {
    let codec = Gs1Codec;
    let decoded = codec
        .decode(ScanInput::new("]d2", b"0109520123456788\x1d17251231"))
        .expect("decode should succeed");
    let got = elements_semantic(&decoded.parsed).expect("expected GS1 elements");
    assert_eq!(
        got,
        vec![
            ("01".to_owned(), "09520123456788".to_owned()),
            ("17".to_owned(), "251231".to_owned()),
        ]
    );
}

#[test]
fn j1_gs_separator_decodes_to_expected_elements() {
    let codec = Gs1Codec;
    let decoded = codec
        .decode(ScanInput::new("]J1", b"0109520123456788\x1d17251231"))
        .expect("decode should succeed");
    let got = elements_semantic(&decoded.parsed).expect("expected GS1 elements");
    assert_eq!(
        got,
        vec![
            ("01".to_owned(), "09520123456788".to_owned()),
            ("17".to_owned(), "251231".to_owned()),
        ]
    );
}

#[test]
fn carrier_separator_legality_matrix() {
    struct Case {
        name: &'static str,
        sym: &'static str,
        payload: &'static [u8],
        should_succeed: bool,
    }

    let cases = [
        Case {
            name: "d2_gs_separator",
            sym: "]d2",
            payload: b"0109520123456788\x1d17251231",
            should_succeed: true,
        },
        Case {
            name: "c1_gs_separator",
            sym: "]C1",
            payload: b"0109520123456788\x1d17251231",
            should_succeed: true,
        },
        Case {
            name: "e0_gs_separator",
            sym: "]e0",
            payload: b"0109520123456788\x1d17251231",
            should_succeed: true,
        },
        Case {
            name: "j1_gs_separator",
            sym: "]J1",
            payload: b"0109520123456788\x1d17251231",
            should_succeed: true,
        },
        Case {
            name: "q3_percent_separator",
            sym: "]Q3",
            payload: b"0109520123456788%17251231",
            should_succeed: true,
        },
        Case {
            name: "q3_gs_separator",
            sym: "]Q3",
            payload: b"0109520123456788\x1d17251231",
            should_succeed: true,
        },
        Case {
            name: "d2_double_separator_rejected",
            sym: "]d2",
            payload: b"0109520123456788\x1d\x1d17251231",
            should_succeed: false,
        },
        Case {
            name: "q3_double_separator_rejected",
            sym: "]Q3",
            payload: b"0109520123456788\x1d\x1d17251231",
            should_succeed: false,
        },
    ];

    let codec = Gs1Codec;
    for case in cases {
        let got = codec.decode(ScanInput::new(case.sym, case.payload));
        if case.should_succeed {
            let decoded = got.unwrap_or_else(|e| panic!("{} should decode: {e}", case.name));
            assert!(
                decoded.parsed.ai_elements().is_some(),
                "{} expected GS1 elements",
                case.name
            );
        } else {
            assert!(got.is_err(), "{} should fail", case.name);
        }
    }
}

#[test]
fn ean13_composite_decode_parses_cc_ai_semantics() {
    let codec = Gs1Codec;
    let decoded = codec
        .decode(ScanInput::new(
            "]E0",
            b"2112345678900|]e099COMPOSITE\x1d98XYZ",
        ))
        .expect("decode should succeed");
    let got = elements_semantic(&decoded.parsed).expect("expected composite CC elements");
    assert_eq!(
        got,
        vec![
            ("99".to_owned(), "COMPOSITE".to_owned()),
            ("98".to_owned(), "XYZ".to_owned()),
        ]
    );
    assert_eq!(decoded.to_hri().as_deref(), Some("(99)COMPOSITE(98)XYZ"));
}

#[test]
fn ean8_composite_decode_rejects_missing_cc_payload() {
    let codec = Gs1Codec;
    let err = codec
        .decode(ScanInput::new("]E4", b"02345673|]e0"))
        .expect_err("decode should fail");
    assert!(matches!(err, AidcError::InvalidPayload(_)));
}
