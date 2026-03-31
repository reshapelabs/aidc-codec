use aidc_core::{AidcError, CanonicalPayload, DataElement};

use crate::ai::{ai_requires_fnc1, lookup_ai, validate_ai_value, validate_message_rules};
use crate::model::TransportKind;

pub fn encode_payload(
    kind: TransportKind,
    payload: CanonicalPayload,
) -> Result<Vec<u8>, AidcError> {
    match (kind, payload) {
        (TransportKind::PlainDigits, CanonicalPayload::Digits(s)) => {
            if !s.as_bytes().iter().all(u8::is_ascii_digit) {
                return Err(AidcError::InvalidPayload(
                    "plain-digit transport must contain only ASCII digits".to_owned(),
                ));
            }
            Ok(s.into_bytes())
        }
        (TransportKind::Gs1ElementString, CanonicalPayload::Elements(elements)) => {
            encode_element_string(elements)
        }
        (TransportKind::Gs1DigitalLinkUri, CanonicalPayload::Elements(elements)) => {
            encode_digital_link(elements)
        }
        (TransportKind::Gs1CompositePacket, _) => Err(AidcError::UnsupportedTransportKind(
            "gs1 composite packet encode not implemented".to_owned(),
        )),
        (TransportKind::PlainDigits, _) => Err(AidcError::InvalidPayload(
            "digits transport requires CanonicalPayload::Digits".to_owned(),
        )),
        (TransportKind::Gs1ElementString, _) => Err(AidcError::InvalidPayload(
            "element-string transport requires CanonicalPayload::Elements".to_owned(),
        )),
        (TransportKind::Gs1DigitalLinkUri, _) => Err(AidcError::InvalidPayload(
            "digital-link transport requires CanonicalPayload::Elements".to_owned(),
        )),
    }
}

fn encode_element_string(elements: Vec<DataElement>) -> Result<Vec<u8>, AidcError> {
    if elements.is_empty() {
        return Err(AidcError::InvalidPayload(
            "element-string payload requires at least one element".to_owned(),
        ));
    }

    let mut out = Vec::<u8>::new();
    validate_message_rules(elements.iter().map(|e| e.id.as_str()))?;
    for (idx, element) in elements.iter().enumerate() {
        if element.id.is_empty() || !element.id.chars().all(|c| c.is_ascii_digit()) {
            return Err(AidcError::InvalidPayload(
                "element id must be a numeric AI code".to_owned(),
            ));
        }
        validate_ai_value(&element.id, &element.value)?;

        if idx > 0 && ai_requires_fnc1(&elements[idx - 1].id) {
            out.push(0x1D);
        }
        out.extend_from_slice(element.id.as_bytes());
        out.extend_from_slice(element.value.as_bytes());
    }
    Ok(out)
}

fn encode_digital_link(elements: Vec<DataElement>) -> Result<Vec<u8>, AidcError> {
    if elements.is_empty() {
        return Err(AidcError::InvalidPayload(
            "digital-link payload requires at least one element".to_owned(),
        ));
    }

    validate_message_rules(elements.iter().map(|e| e.id.as_str()))?;

    let mut primary_index = None;
    let mut seen = std::collections::HashSet::<String>::new();
    for (idx, element) in elements.iter().enumerate() {
        if element.id.is_empty() || !element.id.chars().all(|c| c.is_ascii_digit()) {
            return Err(AidcError::InvalidPayload(
                "element id must be a numeric AI code".to_owned(),
            ));
        }
        let canonical = canonicalized_value(&element.id, &element.value);
        validate_ai_value(&element.id, &canonical)?;
        if !seen.insert(element.id.clone()) {
            return Err(AidcError::InvalidPayload(
                "digital-link encode does not allow repeated AI keys".to_owned(),
            ));
        }
        if lookup_ai(&element.id).is_some_and(|m| m.dl_primary_key) {
            if primary_index.is_some() {
                return Err(AidcError::InvalidPayload(
                    "digital-link encode requires exactly one primary key AI".to_owned(),
                ));
            }
            primary_index = Some(idx);
        }
    }

    let primary_index = primary_index.ok_or_else(|| {
        AidcError::InvalidPayload("digital-link encode requires a primary key AI".to_owned())
    })?;
    let primary = &elements[primary_index];
    let primary_meta = lookup_ai(&primary.id).expect("primary AI must exist in dictionary");
    let qualifier_patterns = primary_meta
        .dl_qualifiers
        .unwrap_or("")
        .split('|')
        .flat_map(|alt| alt.split(','))
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();

    let mut path_elements = vec![(
        primary.id.clone(),
        canonicalized_value(&primary.id, &primary.value),
    )];
    let mut query_elements = Vec::<(String, String)>::new();

    for (idx, element) in elements.iter().enumerate() {
        if idx == primary_index {
            continue;
        }
        let value = canonicalized_value(&element.id, &element.value);
        if qualifier_patterns
            .iter()
            .any(|pattern| matches_ai_pattern(&element.id, pattern))
        {
            path_elements.push((element.id.clone(), value));
            continue;
        }

        let Some(meta) = lookup_ai(&element.id) else {
            return Err(AidcError::InvalidPayload(
                "digital-link encode requires known AI metadata".to_owned(),
            ));
        };
        if !meta.dl_data_attr {
            return Err(AidcError::InvalidPayload(format!(
                "AI {} is not valid as a query data attribute",
                element.id
            )));
        }
        query_elements.push((element.id.clone(), value));
    }

    query_elements.sort_by(|(a, _), (b, _)| a.cmp(b));

    let mut uri = String::from("https://id.gs1.org");
    for (ai, value) in path_elements {
        uri.push('/');
        uri.push_str(&ai);
        uri.push('/');
        uri.push_str(&percent_encode_component(&value));
    }
    if !query_elements.is_empty() {
        uri.push('?');
        for (idx, (ai, value)) in query_elements.iter().enumerate() {
            if idx > 0 {
                uri.push('&');
            }
            uri.push_str(ai);
            uri.push('=');
            uri.push_str(&percent_encode_component(value));
        }
    }
    Ok(uri.into_bytes())
}

fn canonicalized_value(ai: &str, value: &str) -> String {
    if ai == "01" && value.chars().all(|c| c.is_ascii_digit()) && matches!(value.len(), 8 | 12 | 13)
    {
        format!("{value:0>14}")
    } else {
        value.to_owned()
    }
}

fn percent_encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'.' | b'_' | b'~') {
            out.push(*b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

fn matches_ai_pattern(ai: &str, pattern: &str) -> bool {
    if ai.len() != pattern.len() {
        return false;
    }
    ai.as_bytes()
        .iter()
        .zip(pattern.as_bytes())
        .all(|(a, p)| match p {
            b'n' => a.is_ascii_digit(),
            _ => a == p,
        })
}

#[cfg(test)]
mod tests {
    use super::encode_payload;
    use crate::model::TransportKind;
    use aidc_core::{CanonicalPayload, DataElement};

    #[test]
    fn dl_encode_builds_canonical_uri_with_qualifiers_and_sorted_query() {
        let out = encode_payload(
            TransportKind::Gs1DigitalLinkUri,
            CanonicalPayload::Elements(vec![
                DataElement {
                    id: "01".to_owned(),
                    value: "9520123456788".to_owned(),
                },
                DataElement {
                    id: "21".to_owned(),
                    value: "ABC123".to_owned(),
                },
                DataElement {
                    id: "10".to_owned(),
                    value: "BATCH/42".to_owned(),
                },
                DataElement {
                    id: "99".to_owned(),
                    value: "XYZ".to_owned(),
                },
                DataElement {
                    id: "17".to_owned(),
                    value: "201225".to_owned(),
                },
            ]),
        )
        .expect("encode should succeed");
        let uri = String::from_utf8(out).expect("uri utf8");
        assert_eq!(
            uri,
            "https://id.gs1.org/01/09520123456788/21/ABC123/10/BATCH%2F42?17=201225&99=XYZ"
        );
    }

    #[test]
    fn dl_encode_rejects_payload_without_primary_key() {
        let err = encode_payload(
            TransportKind::Gs1DigitalLinkUri,
            CanonicalPayload::Elements(vec![DataElement {
                id: "99".to_owned(),
                value: "LOT42".to_owned(),
            }]),
        )
        .expect_err("encode should fail");
        assert!(err
            .to_string()
            .contains("digital-link encode requires a primary key AI"));
    }

    #[test]
    fn dl_encode_rejects_non_data_attribute_in_query_position() {
        let err = encode_payload(
            TransportKind::Gs1DigitalLinkUri,
            CanonicalPayload::Elements(vec![
                DataElement {
                    id: "01".to_owned(),
                    value: "09520123456788".to_owned(),
                },
                DataElement {
                    id: "7040".to_owned(),
                    value: "1ABC".to_owned(),
                },
            ]),
        )
        .expect_err("encode should fail");
        assert!(err
            .to_string()
            .contains("is not valid as a query data attribute"));
    }
}
