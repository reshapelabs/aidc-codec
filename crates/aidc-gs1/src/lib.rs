mod ai;
pub mod check;
pub mod conformance;
pub mod encode;
pub mod identify;
pub mod model;
pub mod normalize;
pub mod parser;
pub mod variable_measure;

use aidc_core::{AidcError, EncodeInput, EncodedScan, ScanInput, TransportCodec};

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
        let transport = identify_transport(&input.symbology_identifier)?;
        let raw = encode::encode_payload(transport.kind, input.payload)?;
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

pub fn decode_scan(scan: &[u8]) -> Result<ParseResult, AidcError> {
    let input = ScanInput::from_aim_scan(scan)?;
    Gs1Codec.decode(input)
}
