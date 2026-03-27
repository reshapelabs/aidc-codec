#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CarrierFamily {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportKind {
    PlainDigits,
    Gs1ElementString,
    Gs1DigitalLinkUri,
    Gs1CompositePacket,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbologyId {
    E0,
    E1,
    E2,
    E3,
    E4,
    I1,
    C1,
    LowerE0,
    LowerE1,
    LowerE2,
    D2,
    Q3,
    J0,
    J1,
    D1,
    Q1,
    Unknown(String),
}

impl SymbologyId {
    pub fn parse(input: &str) -> Self {
        match input {
            "]E0" => Self::E0,
            "]E1" => Self::E1,
            "]E2" => Self::E2,
            "]E3" => Self::E3,
            "]E4" => Self::E4,
            "]I1" => Self::I1,
            "]C1" => Self::C1,
            "]e0" => Self::LowerE0,
            "]e1" => Self::LowerE1,
            "]e2" => Self::LowerE2,
            "]d2" => Self::D2,
            "]Q3" => Self::Q3,
            "]J0" => Self::J0,
            "]J1" => Self::J1,
            "]d1" => Self::D1,
            "]Q1" => Self::Q1,
            other => Self::Unknown(other.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transport {
    pub symbology_id: SymbologyId,
    pub carrier: CarrierFamily,
    pub kind: TransportKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gs1TransportMessage {
    pub transport: Transport,
    pub normalized: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiElement {
    pub ai: Gs1Ai,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnownAi(&'static AiMeta);

impl KnownAi {
    pub fn parse(ai: &str) -> Option<Self> {
        lookup_ai(ai).map(Self)
    }

    pub fn code(self) -> &'static str {
        self.0.code
    }

    pub fn meta(self) -> &'static AiMeta {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Gs1Ai {
    Known(KnownAi),
    Unknown(String),
}

impl Gs1Ai {
    pub fn parse(ai: &str) -> Self {
        if let Some(k) = KnownAi::parse(ai) {
            Self::Known(k)
        } else {
            Self::Unknown(ai.to_owned())
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::Known(k) => k.code(),
            Self::Unknown(v) => v.as_str(),
        }
    }

    pub fn known(&self) -> Option<KnownAi> {
        match self {
            Self::Known(k) => Some(*k),
            Self::Unknown(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedPayload {
    Digits(String),
    Gs1ElementString {
        original: Vec<u8>,
        elements: Vec<AiElement>,
    },
    Gs1DigitalLink {
        uri: String,
        elements: Vec<AiElement>,
    },
    CompositePacket(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub transport: Transport,
    pub parsed: ParsedPayload,
}

impl ParsedPayload {
    pub fn ai_elements(&self) -> Option<&[AiElement]> {
        match self {
            Self::Gs1ElementString { elements, .. } => Some(elements),
            Self::Gs1DigitalLink { elements, .. } => Some(elements),
            Self::Digits(_) | Self::CompositePacket(_) => None,
        }
    }

    pub fn to_hri(&self) -> Option<String> {
        let elements = self.ai_elements()?;
        let mut out = String::new();
        for e in elements {
            out.push('(');
            out.push_str(e.ai.code());
            out.push(')');
            out.push_str(&e.value);
        }
        Some(out)
    }
}

impl ParseResult {
    pub fn to_hri(&self) -> Option<String> {
        self.parsed.to_hri()
    }
}
use crate::ai::{lookup_ai, AiMeta};
