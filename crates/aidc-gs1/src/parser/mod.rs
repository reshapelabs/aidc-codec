#[cfg(any(
    feature = "gs1-core",
    feature = "gs1-dl",
    feature = "gs1-composite"
))]
mod gs1;

use aidc_core::AidcError;

use crate::model::{Gs1TransportMessage, ParseResult, ParsedPayload, TransportKind};

pub fn parse_payload(message: Gs1TransportMessage) -> Result<ParseResult, AidcError> {
    match message.transport.kind {
        TransportKind::PlainDigits => Ok(ParseResult {
            transport: message.transport,
            parsed: ParsedPayload::Digits(String::from_utf8(message.normalized).map_err(|_| {
                AidcError::InvalidPayload("digits were not valid UTF-8".to_owned())
            })?),
        }),
        TransportKind::Gs1ElementString => {
            #[cfg(feature = "gs1-core")]
            {
                gs1::parse_element_string(message.transport, message.normalized)
            }
            #[cfg(not(feature = "gs1-core"))]
            {
                Err(AidcError::UnsupportedTransportKind(
                    "gs1 element string".to_owned(),
                ))
            }
        }
        TransportKind::Gs1DigitalLinkUri => {
            #[cfg(feature = "gs1-dl")]
            {
                gs1::parse_digital_link(message.transport, message.normalized)
            }
            #[cfg(not(feature = "gs1-dl"))]
            {
                Err(AidcError::UnsupportedTransportKind(
                    "gs1 digital link".to_owned(),
                ))
            }
        }
        TransportKind::Gs1CompositePacket => {
            #[cfg(feature = "gs1-composite")]
            {
                gs1::parse_composite_packet(message.transport, message.normalized)
            }
            #[cfg(not(feature = "gs1-composite"))]
            {
                Err(AidcError::UnsupportedTransportKind(
                    "gs1 composite packet".to_owned(),
                ))
            }
        }
    }
}
