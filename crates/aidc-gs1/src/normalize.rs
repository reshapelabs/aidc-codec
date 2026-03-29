use aidc_core::AidcError;

use crate::model::{SymbologyId, Transport, TransportKind};

pub fn normalize_payload(transport: &Transport, raw: &[u8]) -> Result<Vec<u8>, AidcError> {
    match transport.kind {
        TransportKind::PlainDigits => normalize_digits(raw),
        TransportKind::Gs1ElementString => normalize_element_string(transport, raw),
        TransportKind::Gs1DigitalLinkUri => Ok(raw.to_vec()),
        TransportKind::Gs1CompositePacket => Ok(raw.to_vec()),
    }
}

fn normalize_element_string(transport: &Transport, raw: &[u8]) -> Result<Vec<u8>, AidcError> {
    if matches!(transport.symbology_id, SymbologyId::Q3) && !raw.contains(&0x1d) && raw.contains(&b'%') {
        return Ok(raw
            .iter()
            .map(|b| if *b == b'%' { 0x1d } else { *b })
            .collect());
    }
    Ok(raw.to_vec())
}

fn normalize_digits(raw: &[u8]) -> Result<Vec<u8>, AidcError> {
    if raw.iter().all(u8::is_ascii_digit) {
        return Ok(raw.to_vec());
    }

    Err(AidcError::InvalidPayload(
        "plain-digit transport must contain only ASCII digits".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::normalize_payload;
    use crate::model::{CarrierFamily, SymbologyId, Transport, TransportKind};

    #[test]
    fn q3_maps_percent_to_fnc1_when_no_gs_present() {
        let transport = Transport {
            symbology_id: SymbologyId::Q3,
            carrier: CarrierFamily::Gs1Qr,
            kind: TransportKind::Gs1ElementString,
        };
        let normalized = normalize_payload(&transport, b"0109520123456788%17251231")
            .expect("normalize should succeed");
        assert_eq!(normalized, b"0109520123456788\x1d17251231");
    }

    #[test]
    fn q3_keeps_percent_when_gs_already_present() {
        let transport = Transport {
            symbology_id: SymbologyId::Q3,
            carrier: CarrierFamily::Gs1Qr,
            kind: TransportKind::Gs1ElementString,
        };
        let normalized = normalize_payload(&transport, b"10ABC%123\x1d17251231")
            .expect("normalize should succeed");
        assert_eq!(normalized, b"10ABC%123\x1d17251231");
    }
}
