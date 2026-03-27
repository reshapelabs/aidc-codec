use aidc_core::AidcError;

use crate::model::{CarrierFamily, SymbologyId, Transport, TransportKind};

pub fn identify_transport(symbology_identifier: &str) -> Result<Transport, AidcError> {
    let symbology_id = SymbologyId::parse(symbology_identifier);

    let transport = match symbology_id {
        SymbologyId::E0
        | SymbologyId::E1
        | SymbologyId::E2
        | SymbologyId::E3
        | SymbologyId::E4 => Transport {
            symbology_id,
            carrier: CarrierFamily::EanUpc,
            kind: TransportKind::PlainDigits,
        },
        SymbologyId::I1 => Transport {
            symbology_id,
            carrier: CarrierFamily::Itf,
            kind: TransportKind::PlainDigits,
        },
        SymbologyId::C1 => Transport {
            symbology_id,
            carrier: CarrierFamily::Gs1_128,
            kind: TransportKind::Gs1ElementString,
        },
        SymbologyId::LowerE0 => Transport {
            symbology_id,
            carrier: CarrierFamily::Gs1Databar,
            kind: TransportKind::Gs1ElementString,
        },
        SymbologyId::LowerE1 | SymbologyId::LowerE2 => Transport {
            symbology_id,
            carrier: CarrierFamily::Gs1Composite,
            kind: TransportKind::Gs1CompositePacket,
        },
        SymbologyId::D2 => Transport {
            symbology_id,
            carrier: CarrierFamily::Gs1DataMatrix,
            kind: TransportKind::Gs1ElementString,
        },
        SymbologyId::Q3 => Transport {
            symbology_id,
            carrier: CarrierFamily::Gs1Qr,
            kind: TransportKind::Gs1ElementString,
        },
        SymbologyId::J1 => Transport {
            symbology_id,
            carrier: CarrierFamily::Gs1DotCode,
            kind: TransportKind::Gs1ElementString,
        },
        SymbologyId::J0 => {
            return Err(AidcError::UnsupportedSymbologyId("]J0".to_owned()));
        }
        SymbologyId::D1 => Transport {
            symbology_id,
            carrier: CarrierFamily::DataMatrix,
            kind: TransportKind::Gs1DigitalLinkUri,
        },
        SymbologyId::Q1 => Transport {
            symbology_id,
            carrier: CarrierFamily::Qr,
            kind: TransportKind::Gs1DigitalLinkUri,
        },
        SymbologyId::Unknown(value) => return Err(AidcError::UnsupportedSymbologyId(value)),
    };

    Ok(transport)
}
