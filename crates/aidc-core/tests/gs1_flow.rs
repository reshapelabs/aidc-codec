use aidc_core::{Codec, ScanInput};
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
            assert_eq!(elements[0].ai, "raw");
            assert_eq!(elements[0].value, "0109520123456788");
        }
        other => panic!("unexpected parsed payload: {other:?}"),
    }
}

#[test]
fn decode_scan_splits_aim_prefix() {
    let result = decode_scan(b"]d20109520123456788").expect("decode should succeed");
    assert_eq!(result.transport.symbology_id, SymbologyId::D2);
}
