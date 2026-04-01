use aidc_core::{AidcError, CanonicalPayload, DataElement, EncodeInput, ScanInput, TransportCodec};
use aidc_gs1::{Gs1Codec, ParsedPayload};

#[test]
fn unsupported_symbology_error_contract_is_stable() {
    let codec = Gs1Codec;
    let err = codec
        .decode(ScanInput::new("]Z9", b"123"))
        .expect_err("decode should fail");
    assert!(matches!(err, AidcError::UnsupportedSymbologyId(_)));
    assert_eq!(err.to_string(), "unsupported symbology identifier: ]Z9");
}

#[test]
fn e0_elements_encode_routes_to_composite_transport_stably() {
    let codec = Gs1Codec;
    let encoded = codec
        .encode(EncodeInput {
            symbology_identifier: "]E0".to_owned(),
            payload: CanonicalPayload::Elements(vec![
                DataElement {
                    id: "01".to_owned(),
                    value: "02112345678900".to_owned(),
                },
                DataElement {
                    id: "99".to_owned(),
                    value: "ABC".to_owned(),
                },
            ]),
        })
        .expect("encode should succeed");
    assert_eq!(encoded.symbology_identifier, "]E0");
    assert_eq!(encoded.raw, b"2112345678900|]e099ABC");

    let decoded = codec
        .decode(ScanInput::new(&encoded.symbology_identifier, &encoded.raw))
        .expect("decode should succeed");
    match decoded.parsed {
        ParsedPayload::CompositePacket { linear, .. } => assert_eq!(linear, "2112345678900"),
        other => panic!("expected CompositePacket, got {other:?}"),
    }
}

#[test]
fn lowere1_elements_encode_contract_requires_explicit_composite_payload() {
    let codec = Gs1Codec;
    let err = codec
        .encode(EncodeInput {
            symbology_identifier: "]e1".to_owned(),
            payload: CanonicalPayload::Elements(vec![
                DataElement {
                    id: "01".to_owned(),
                    value: "09520123456788".to_owned(),
                },
                DataElement {
                    id: "99".to_owned(),
                    value: "ABC".to_owned(),
                },
            ]),
        })
        .expect_err("encode should fail");
    assert!(matches!(err, AidcError::UnsupportedTransportKind(_)));
    assert_eq!(
        err.to_string(),
        "unsupported transport kind: composite encode is only supported for ]E0 and ]E4"
    );
}

#[test]
fn lowere1_composite_payload_roundtrip_contract_is_stable() {
    let codec = Gs1Codec;
    let encoded = codec
        .encode(EncodeInput {
            symbology_identifier: "]e1".to_owned(),
            payload: CanonicalPayload::Composite {
                linear: "2112345678900".to_owned(),
                elements: vec![
                    DataElement {
                        id: "99".to_owned(),
                        value: "ABC".to_owned(),
                    },
                    DataElement {
                        id: "98".to_owned(),
                        value: "XYZ".to_owned(),
                    },
                ],
            },
        })
        .expect("encode should succeed");
    assert_eq!(encoded.symbology_identifier, "]e1");
    assert_eq!(encoded.raw, b"2112345678900|]e099ABC\x1d98XYZ");
}
