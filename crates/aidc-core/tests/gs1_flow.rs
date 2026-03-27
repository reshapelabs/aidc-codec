use aidc_core::{CanonicalPayload, DataElement, EncodeInput, ScanInput, TransportCodec};
use aidc_gs1::{decode_scan, Gs1Codec, ParsedPayload, SymbologyId};

#[test]
fn codec_trait_flow_decodes_aim_payload() {
    let codec = Gs1Codec;
    let result = codec
        .decode(ScanInput::new("]d2", b"0109520123456788"))
        .expect("decode should succeed");

    assert_eq!(result.transport.symbology_id, SymbologyId::D2);
    match result.parsed {
        ParsedPayload::Gs1ElementString { elements, .. } => {
            assert_eq!(elements.len(), 1);
            assert_eq!(elements[0].ai.code(), "01");
            assert_eq!(elements[0].value, "09520123456788");
        }
        other => panic!("unexpected parsed payload: {other:?}"),
    }
}

#[test]
fn decode_scan_splits_aim_prefix() {
    let result = decode_scan(b"]d20109520123456788").expect("decode should succeed");
    assert_eq!(result.transport.symbology_id, SymbologyId::D2);
}

#[test]
fn encoder_codec_emits_gs1_element_string_payload() {
    let codec = Gs1Codec;
    let out = codec
        .encode(EncodeInput {
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
            ]),
        })
        .expect("encode should succeed");

    assert_eq!(out.symbology_identifier, "]d2");
    assert_eq!(out.raw, b"010952012345678810ABC123");
}

#[test]
fn encoder_codec_inserts_fnc1_after_variable_ai() {
    let codec = Gs1Codec;
    let out = codec
        .encode(EncodeInput {
            symbology_identifier: "]d2".to_owned(),
            payload: CanonicalPayload::Elements(vec![
                DataElement {
                    id: "10".to_owned(),
                    value: "LOT1".to_owned(),
                },
                DataElement {
                    id: "17".to_owned(),
                    value: "251231".to_owned(),
                },
            ]),
        })
        .expect("encode should succeed");

    assert_eq!(out.raw, b"10LOT1\x1d17251231");
}
