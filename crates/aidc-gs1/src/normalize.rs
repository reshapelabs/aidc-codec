use aidc_core::AidcError;

use crate::model::{SymbologyId, Transport, TransportKind};

pub fn normalize_payload(transport: &Transport, raw: &[u8]) -> Result<Vec<u8>, AidcError> {
    match transport.kind {
        TransportKind::PlainDigits => normalize_digits(transport, raw),
        TransportKind::Gs1ElementString => normalize_element_string(transport, raw),
        TransportKind::Gs1DigitalLinkUri => Ok(raw.to_vec()),
        TransportKind::Gs1CompositePacket => Ok(raw.to_vec()),
    }
}

fn normalize_element_string(transport: &Transport, raw: &[u8]) -> Result<Vec<u8>, AidcError> {
    if matches!(transport.symbology_id, SymbologyId::Q3)
        && !raw.contains(&0x1d)
        && raw.contains(&b'%')
    {
        return Ok(raw
            .iter()
            .map(|b| if *b == b'%' { 0x1d } else { *b })
            .collect());
    }
    Ok(raw.to_vec())
}

fn normalize_digits(transport: &Transport, raw: &[u8]) -> Result<Vec<u8>, AidcError> {
    if matches!(transport.symbology_id, SymbologyId::E0 | SymbologyId::E4)
        && raw.windows(4).any(|w| w == b"|]e0")
    {
        let text = std::str::from_utf8(raw).map_err(|_| {
            AidcError::InvalidPayload("EAN/UPC composite payload must be valid UTF-8".to_owned())
        })?;
        let (primary, cc) = text.split_once("|]e0").ok_or_else(|| {
            AidcError::InvalidPayload("EAN/UPC composite payload missing '|]e0'".to_owned())
        })?;
        let expected_len = if matches!(transport.symbology_id, SymbologyId::E0) {
            13usize
        } else {
            8usize
        };
        if primary.len() != expected_len || !primary.as_bytes().iter().all(u8::is_ascii_digit) {
            return Err(AidcError::InvalidPayload(
                "invalid EAN primary data".to_owned(),
            ));
        }
        if !has_valid_mod10(primary.as_bytes()) {
            return Err(AidcError::InvalidPayload(
                "invalid EAN check digit".to_owned(),
            ));
        }
        if cc.is_empty() {
            return Err(AidcError::InvalidPayload(
                "empty composite payload".to_owned(),
            ));
        }
        if text.matches("|]e0").count() > 1 {
            return Err(AidcError::InvalidPayload(
                "composite payload must contain one '|]e0' separator".to_owned(),
            ));
        }
        if cc.contains('^') {
            return Err(AidcError::InvalidPayload(
                "raw '^' is not valid in composite payload".to_owned(),
            ));
        }
        return Ok(raw.to_vec());
    }

    if !raw.iter().all(u8::is_ascii_digit) {
        return Err(AidcError::InvalidPayload(
            "plain-digit transport must contain only ASCII digits".to_owned(),
        ));
    }

    if matches!(transport.symbology_id, SymbologyId::I1) {
        if raw.len() != 14 {
            return Err(AidcError::InvalidPayload(
                "ITF-14 payload must contain exactly 14 digits".to_owned(),
            ));
        }
        if !has_valid_mod10(raw) {
            return Err(AidcError::InvalidPayload(
                "ITF-14 payload has invalid check digit".to_owned(),
            ));
        }
    }

    Ok(raw.to_vec())
}

fn has_valid_mod10(raw: &[u8]) -> bool {
    if raw.len() < 2 || !raw.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let mut sum = 0u32;
    for (idx, b) in raw[..raw.len() - 1].iter().rev().enumerate() {
        let d = u32::from(*b - b'0');
        sum += if idx % 2 == 0 { 3 * d } else { d };
    }
    let check = (10 - (sum % 10)) % 10;
    raw[raw.len() - 1] == (b'0' + u8::try_from(check).expect("single digit"))
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

    #[test]
    fn i1_accepts_valid_itf14() {
        let transport = Transport {
            symbology_id: SymbologyId::I1,
            carrier: CarrierFamily::Itf,
            kind: TransportKind::PlainDigits,
        };
        let normalized =
            normalize_payload(&transport, b"09520123456788").expect("normalize should succeed");
        assert_eq!(normalized, b"09520123456788");
    }

    #[test]
    fn i1_rejects_wrong_length() {
        let transport = Transport {
            symbology_id: SymbologyId::I1,
            carrier: CarrierFamily::Itf,
            kind: TransportKind::PlainDigits,
        };
        let err =
            normalize_payload(&transport, b"9520123456788").expect_err("normalize should fail");
        assert!(err.to_string().contains("exactly 14 digits"));
    }

    #[test]
    fn i1_rejects_bad_check_digit() {
        let transport = Transport {
            symbology_id: SymbologyId::I1,
            carrier: CarrierFamily::Itf,
            kind: TransportKind::PlainDigits,
        };
        let err =
            normalize_payload(&transport, b"09520123456789").expect_err("normalize should fail");
        assert!(err.to_string().contains("invalid check digit"));
    }
}
