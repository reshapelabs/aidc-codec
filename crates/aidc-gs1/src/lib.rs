//! GS1 codec crate for transport identification, normalization, decoding, and encoding.
//!
//! Decode entrypoints:
//! - [`decode_scan`]: strict AIM scan bytes (`]d2...`, `]Q3...`, `]E0...`)
//! - [`decode_aim_str`]: strict AIM scan string convenience wrapper
//!
//! ```no_run
//! use aidc_core::{CanonicalPayload, DataElement, EncodeInput, TransportCodec};
//! use aidc_gs1::Gs1Codec;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let codec = Gs1Codec;
//!     let encoded = codec.encode(EncodeInput {
//!         symbology_identifier: "]d2".to_owned(),
//!         payload: CanonicalPayload::Elements(vec![
//!             DataElement {
//!                 id: "01".to_owned(),
//!                 value: "09520123456788".to_owned(),
//!             },
//!             DataElement {
//!                 id: "17".to_owned(),
//!                 value: "250101".to_owned(),
//!             },
//!         ]),
//!     })?;
//!
//!     let mut scan = encoded.symbology_identifier.into_bytes();
//!     scan.extend_from_slice(&encoded.raw);
//!     let decoded = aidc_gs1::decode_scan(&scan)?;
//!     assert_eq!(decoded.to_hri().as_deref(), Some("(01)09520123456788(17)250101"));
//!     Ok(())
//! }
//! ```

mod ai;
pub mod check;
pub mod conformance;
pub mod encode;
pub mod identify;
pub mod model;
pub mod normalize;
pub mod parser;
pub mod variable_measure;
#[cfg(feature = "wire-record")]
pub mod wire;

use aidc_core::{AidcError, EncodeInput, EncodedScan, ScanInput, TransportCodec};

pub use ai::{dictionary_source_provenance, DictionarySourceProvenance};
pub use conformance::{
    parse_bracketed_ai, parse_dl_uri, process_scan_data, DlParseOptions, ScanDataOutcome,
};
pub use identify::identify_transport;
pub use model::{
    AiElement, CarrierFamily, Gs1Ai, Gs1TransportMessage, KnownAi, ParseResult, ParsedPayload,
    SymbologyId, Transport, TransportKind,
};
pub use normalize::normalize_payload;
pub use parser::parse_payload;
#[cfg(feature = "wire-record")]
pub use wire::{
    AiElementRecord, CarrierFamilyRecord, ParsedPayloadRecord, ParsedScanRecord,
    TransportKindRecord, TransportRecord, PARSED_SCAN_RECORD_SCHEMA_VERSION,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Gs1Codec;

impl TransportCodec for Gs1Codec {
    type TransportMsg = Gs1TransportMessage;
    type Decoded = ParseResult;
    type EncodeRequest = EncodeInput;

    fn decode_transport(&self, input: ScanInput<'_>) -> Result<Self::TransportMsg, AidcError> {
        let transport = identify_transport(input.symbology_identifier)?;
        let normalized = normalize_payload(&transport, input.raw)?;

        Ok(Gs1TransportMessage {
            transport,
            normalized,
        })
    }

    fn parse_payload(&self, message: Self::TransportMsg) -> Result<Self::Decoded, AidcError> {
        parse_payload(message)
    }

    fn format_payload(&self, input: Self::EncodeRequest) -> Result<Self::TransportMsg, AidcError> {
        let mut transport = identify_transport(&input.symbology_identifier)?;
        if matches!(transport.symbology_id, SymbologyId::E0 | SymbologyId::E4)
            && matches!(
                input.payload,
                aidc_core::CanonicalPayload::Elements(_)
                    | aidc_core::CanonicalPayload::Composite { .. }
            )
        {
            transport.carrier = CarrierFamily::Gs1Composite;
            transport.kind = TransportKind::Gs1CompositePacket;
        }
        let raw = encode::encode_payload(&transport, input.payload)?;
        let normalized = normalize_payload(&transport, &raw)?;
        Ok(Gs1TransportMessage {
            transport,
            normalized,
        })
    }

    fn encode_transport(&self, message: Self::TransportMsg) -> Result<EncodedScan, AidcError> {
        Ok(EncodedScan {
            symbology_identifier: message.transport.symbology_id.as_str().to_owned(),
            raw: message.normalized,
        })
    }
}

/// Decodes a strict AIM scan payload.
///
/// The input must start with a 3-byte AIM symbology identifier (`]xx`).
///
/// Accepted examples:
/// - `b"]d20109520123456788"`
/// - `b"]Q3https://id.gs1.org/01/09520123456788"`
///
/// Rejected examples:
/// - `b"0109520123456788"` (no AIM symbology identifier)
/// - `b"(01)09520123456788"` (HRI/bracketed input)
pub fn decode_scan(scan: &[u8]) -> Result<ParseResult, AidcError> {
    let input = ScanInput::from_aim_scan(scan)?;
    Gs1Codec.decode(input)
}

/// Decodes a strict AIM scan string.
///
/// This is equivalent to [`decode_scan`] for UTF-8 string input.
pub fn decode_aim_str(scan: &str) -> Result<ParseResult, AidcError> {
    decode_scan(scan.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::{decode_aim_str, decode_scan};

    #[test]
    fn decode_aim_str_matches_decode_scan() {
        let scan = "]E02112345678900";
        let from_str = decode_aim_str(scan).expect("string decode should succeed");
        let from_bytes = decode_scan(scan.as_bytes()).expect("bytes decode should succeed");
        assert_eq!(from_str, from_bytes);
    }

    #[test]
    fn decode_aim_str_rejects_non_aim_input() {
        let err = decode_aim_str("(01)09520123456788").expect_err("non-AIM input should fail");
        assert_eq!(
            err.to_string(),
            "invalid input: AIM scan must start with ']'",
        );
    }
}
