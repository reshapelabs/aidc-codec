use serde::{Deserialize, Serialize};

use crate::ai::lookup_ai;
use crate::model::{
    AiElement, CarrierFamily, ParseResult, ParsedPayload, Transport, TransportKind,
};

pub const PARSED_SCAN_RECORD_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedScanRecord {
    pub schema_version: u16,
    pub transport: TransportRecord,
    pub parsed: ParsedPayloadRecord,
    pub hri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportRecord {
    pub symbology_id: String,
    pub carrier: CarrierFamilyRecord,
    pub transport_kind: TransportKindRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiElementRecord {
    pub ai: String,
    pub value: String,
    pub known: bool,
    pub private_use: bool,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum ParsedPayloadRecord {
    Digits {
        digits: String,
    },
    Gs1ElementString {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "base64_bytes_option"
        )]
        original: Option<Vec<u8>>,
        elements: Vec<AiElementRecord>,
    },
    Gs1DigitalLink {
        uri: String,
        elements: Vec<AiElementRecord>,
    },
    CompositePacket {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "base64_bytes_option"
        )]
        original: Option<Vec<u8>>,
        linear: String,
        cc_elements: Vec<AiElementRecord>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarrierFamilyRecord {
    EanUpc,
    Itf,
    Gs1_128,
    Gs1Databar,
    Gs1Composite,
    Gs1DataMatrix,
    Gs1Qr,
    Gs1DotCode,
    DataMatrix,
    Qr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKindRecord {
    PlainDigits,
    Gs1ElementString,
    Gs1DigitalLinkUri,
    Gs1CompositePacket,
}

impl From<&ParseResult> for ParsedScanRecord {
    fn from(value: &ParseResult) -> Self {
        Self {
            schema_version: PARSED_SCAN_RECORD_SCHEMA_VERSION,
            transport: (&value.transport).into(),
            parsed: (&value.parsed).into(),
            hri: value.to_hri(),
        }
    }
}

impl From<ParseResult> for ParsedScanRecord {
    fn from(value: ParseResult) -> Self {
        (&value).into()
    }
}

impl From<&Transport> for TransportRecord {
    fn from(value: &Transport) -> Self {
        Self {
            symbology_id: value.symbology_id.as_str().to_owned(),
            carrier: value.carrier.into(),
            transport_kind: value.kind.into(),
        }
    }
}

impl From<Transport> for TransportRecord {
    fn from(value: Transport) -> Self {
        (&value).into()
    }
}

impl From<&AiElement> for AiElementRecord {
    fn from(value: &AiElement) -> Self {
        let code = value.ai.code();
        let known_meta = lookup_ai(code);
        let private_use = is_private_use_ai(code);
        Self {
            ai: code.to_owned(),
            value: value.value.clone(),
            known: known_meta.is_some(),
            private_use,
            display_name: if private_use {
                None
            } else {
                known_meta.and_then(|m| m.title.map(str::to_owned))
            },
        }
    }
}

impl From<AiElement> for AiElementRecord {
    fn from(value: AiElement) -> Self {
        (&value).into()
    }
}

impl From<&ParsedPayload> for ParsedPayloadRecord {
    fn from(value: &ParsedPayload) -> Self {
        match value {
            ParsedPayload::Digits(digits) => Self::Digits {
                digits: digits.clone(),
            },
            ParsedPayload::Gs1ElementString { original, elements } => Self::Gs1ElementString {
                original: Some(original.clone()),
                elements: elements.iter().map(Into::into).collect(),
            },
            ParsedPayload::Gs1DigitalLink { uri, elements } => Self::Gs1DigitalLink {
                uri: uri.clone(),
                elements: elements.iter().map(Into::into).collect(),
            },
            ParsedPayload::CompositePacket {
                original,
                linear,
                cc_elements,
            } => Self::CompositePacket {
                original: Some(original.clone()),
                linear: linear.clone(),
                cc_elements: cc_elements.iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<ParsedPayload> for ParsedPayloadRecord {
    fn from(value: ParsedPayload) -> Self {
        (&value).into()
    }
}

impl From<CarrierFamily> for CarrierFamilyRecord {
    fn from(value: CarrierFamily) -> Self {
        match value {
            CarrierFamily::EanUpc => Self::EanUpc,
            CarrierFamily::Itf => Self::Itf,
            CarrierFamily::Gs1_128 => Self::Gs1_128,
            CarrierFamily::Gs1Databar => Self::Gs1Databar,
            CarrierFamily::Gs1Composite => Self::Gs1Composite,
            CarrierFamily::Gs1DataMatrix => Self::Gs1DataMatrix,
            CarrierFamily::Gs1Qr => Self::Gs1Qr,
            CarrierFamily::Gs1DotCode => Self::Gs1DotCode,
            CarrierFamily::DataMatrix => Self::DataMatrix,
            CarrierFamily::Qr => Self::Qr,
        }
    }
}

impl From<TransportKind> for TransportKindRecord {
    fn from(value: TransportKind) -> Self {
        match value {
            TransportKind::PlainDigits => Self::PlainDigits,
            TransportKind::Gs1ElementString => Self::Gs1ElementString,
            TransportKind::Gs1DigitalLinkUri => Self::Gs1DigitalLinkUri,
            TransportKind::Gs1CompositePacket => Self::Gs1CompositePacket,
        }
    }
}

fn is_private_use_ai(ai: &str) -> bool {
    ai.len() == 2 && ai.parse::<u8>().is_ok_and(|code| (91..=99).contains(&code))
}

mod base64_bytes_option {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(bytes) => {
                serializer.serialize_some(&base64::engine::general_purpose::STANDARD.encode(bytes))
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = Option::<String>::deserialize(deserializer)?;
        encoded
            .map(|v| base64::engine::general_purpose::STANDARD.decode(v))
            .transpose()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::{CarrierFamilyRecord, ParsedScanRecord, TransportKindRecord};
    use crate::model::{
        AiElement, CarrierFamily, Gs1Ai, ParseResult, ParsedPayload, SymbologyId, Transport,
        TransportKind,
    };
    use serde_json::json;

    fn sample_parse_result() -> ParseResult {
        ParseResult {
            transport: Transport {
                symbology_id: SymbologyId::D2,
                carrier: CarrierFamily::Gs1DataMatrix,
                kind: TransportKind::Gs1ElementString,
            },
            parsed: ParsedPayload::Gs1ElementString {
                original: b"0109520123456788".to_vec(),
                elements: vec![AiElement {
                    ai: Gs1Ai::parse("01"),
                    value: "09520123456788".to_owned(),
                }],
            },
        }
    }

    #[test]
    fn parsed_record_uses_canonical_codes() {
        let record = ParsedScanRecord::from(sample_parse_result());
        assert_eq!(record.schema_version, 1);
        assert_eq!(record.transport.symbology_id, "]d2");
        assert_eq!(record.transport.carrier, CarrierFamilyRecord::Gs1DataMatrix);
        assert_eq!(
            record.transport.transport_kind,
            TransportKindRecord::Gs1ElementString
        );
    }

    #[test]
    fn record_serializes_with_tagged_payload() {
        let record: ParsedScanRecord = sample_parse_result().into();

        let json = serde_json::to_string(&record).expect("record should serialize");
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"symbology_id\":\"]d2\""));
        assert!(json.contains("\"carrier\":\"gs1_data_matrix\""));
        assert!(json.contains("\"transport_kind\":\"gs1_element_string\""));
        assert!(json.contains("\"format\":\"gs1_element_string\""));
    }

    #[test]
    fn parse_result_into_record_preserves_hri() {
        let parsed: ParseResult = sample_parse_result();
        let record = ParsedScanRecord::from(parsed);
        assert_eq!(record.hri.as_deref(), Some("(01)09520123456788"));
    }

    #[test]
    fn record_can_omit_original_bytes_in_json() {
        let mut record = ParsedScanRecord::from(sample_parse_result());
        if let super::ParsedPayloadRecord::Gs1ElementString { original, .. } = &mut record.parsed {
            *original = None;
        }
        let value = serde_json::to_value(record).expect("record should serialize");
        let parsed = value
            .get("parsed")
            .and_then(serde_json::Value::as_object)
            .expect("parsed object should exist");
        assert!(!parsed.contains_key("original"));
    }

    #[test]
    fn golden_json_gs1_element_string_record() {
        let record = ParsedScanRecord::from(sample_parse_result());
        let value = serde_json::to_value(record).expect("record should serialize");

        let expected = json!({
            "schema_version": 1,
            "transport": {
                "symbology_id": "]d2",
                "carrier": "gs1_data_matrix",
                "transport_kind": "gs1_element_string"
            },
            "parsed": {
                "format": "gs1_element_string",
                "original": "MDEwOTUyMDEyMzQ1Njc4OA==",
                "elements": [
                    {
                        "ai": "01",
                        "value": "09520123456788",
                        "known": true,
                        "private_use": false,
                        "display_name": "GTIN"
                    }
                ]
            },
            "hri": "(01)09520123456788"
        });
        assert_eq!(value, expected);
    }

    #[test]
    fn golden_json_digital_link_record() {
        let parsed = ParseResult {
            transport: Transport {
                symbology_id: SymbologyId::Q3,
                carrier: CarrierFamily::Gs1Qr,
                kind: TransportKind::Gs1DigitalLinkUri,
            },
            parsed: ParsedPayload::Gs1DigitalLink {
                uri: "https://id.gs1.org/01/09520123456788".to_owned(),
                elements: vec![AiElement {
                    ai: Gs1Ai::parse("01"),
                    value: "09520123456788".to_owned(),
                }],
            },
        };
        let record = ParsedScanRecord::from(parsed);
        let value = serde_json::to_value(record).expect("record should serialize");

        let expected = json!({
            "schema_version": 1,
            "transport": {
                "symbology_id": "]Q3",
                "carrier": "gs1_qr",
                "transport_kind": "gs1_digital_link_uri"
            },
            "parsed": {
                "format": "gs1_digital_link",
                "uri": "https://id.gs1.org/01/09520123456788",
                "elements": [
                    {
                        "ai": "01",
                        "value": "09520123456788",
                        "known": true,
                        "private_use": false,
                        "display_name": "GTIN"
                    }
                ]
            },
            "hri": "(01)09520123456788"
        });
        assert_eq!(value, expected);
    }

    #[test]
    fn private_use_ai_hides_display_name() {
        let elem = AiElement {
            ai: Gs1Ai::parse("91"),
            value: "INTERNAL".to_owned(),
        };
        let record = super::AiElementRecord::from(elem);
        assert!(record.known);
        assert!(record.private_use);
        assert_eq!(record.display_name, None);
    }
}
