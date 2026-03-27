pub mod conformance;
pub mod identify;
pub mod model;
pub mod normalize;
pub mod parser;

use aidc_core::{AidcError, Codec, PayloadParser, ScanInput, TransportDecoder};

pub use identify::identify_transport;
pub use model::{
    AiElement, CarrierFamily, Gs1TransportMessage, ParseResult, ParsedPayload, SymbologyId,
    Transport, TransportKind,
};
pub use normalize::normalize_payload;
pub use parser::parse_payload;
pub use conformance::{
    parse_bracketed_ai, parse_dl_uri, process_scan_data, DlParseOptions, ScanDataOutcome,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Gs1Codec;

impl TransportDecoder for Gs1Codec {
    type Message = Gs1TransportMessage;

    fn decode_transport(&self, input: ScanInput<'_>) -> Result<Self::Message, AidcError> {
        let transport = identify_transport(input.symbology_identifier)?;
        let normalized = normalize_payload(&transport, input.raw)?;

        Ok(Gs1TransportMessage {
            transport,
            normalized,
        })
    }
}

impl PayloadParser<Gs1TransportMessage> for Gs1Codec {
    type Output = ParseResult;

    fn parse_payload(&self, message: Gs1TransportMessage) -> Result<Self::Output, AidcError> {
        parse_payload(message)
    }
}

impl Codec for Gs1Codec {
    type Output = ParseResult;

    fn decode(&self, input: ScanInput<'_>) -> Result<Self::Output, AidcError> {
        let message = self.decode_transport(input)?;
        self.parse_payload(message)
    }
}

pub fn decode_scan(scan: &[u8]) -> Result<ParseResult, AidcError> {
    let input = ScanInput::from_aim_scan(scan)?;
    Gs1Codec.decode(input)
}
