use aidc_core::AidcError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiCharset {
    N,
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AiComponent {
    pub charset: AiCharset,
    pub min: u8,
    pub max: u8,
    pub optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AiMeta {
    pub code: &'static str,
    pub fixed_len: Option<u8>,
    pub components: &'static [AiComponent],
    pub fnc1_required: bool,
    pub dl_data_attr: bool,
    pub dl_primary_key: bool,
    pub dl_qualifiers: Option<&'static str>,
}

include!(concat!(env!("OUT_DIR"), "/ai_dictionary.rs"));

pub(crate) fn lookup_ai(ai: &str) -> Option<&'static AiMeta> {
    AI_DICTIONARY.get(ai)
}

pub(crate) fn fixed_value_length(ai: &str) -> Option<usize> {
    lookup_ai(ai).and_then(|m| m.fixed_len.map(usize::from))
}

pub(crate) fn is_known_ai(ai: &str) -> bool {
    lookup_ai(ai).is_some()
}

pub(crate) fn is_primary_ai(ai: &str) -> bool {
    lookup_ai(ai).is_some_and(|m| m.dl_primary_key)
}

pub(crate) fn is_dl_attribute_ai(ai: &str) -> bool {
    lookup_ai(ai).is_some_and(|m| m.dl_data_attr)
}

pub(crate) fn ai_requires_fnc1(ai: &str) -> bool {
    lookup_ai(ai).is_none_or(|m| m.fnc1_required)
}

pub(crate) fn validate_ai_value(ai: &str, value: &str) -> Result<(), AidcError> {
    if value.is_empty() {
        return Err(AidcError::InvalidPayload("AI value must not be empty".to_owned()));
    }
    if value.len() > 90 {
        return Err(AidcError::InvalidPayload("AI value too long".to_owned()));
    }

    let Some(meta) = lookup_ai(ai) else {
        return Ok(());
    };
    validate_components(meta.components, value)
}

fn validate_components(components: &[AiComponent], value: &str) -> Result<(), AidcError> {
    let total_chars = value.chars().count();
    let mut offset = 0usize;

    for (idx, comp) in components.iter().enumerate() {
        let rem = total_chars.saturating_sub(offset);
        if comp.optional && rem == 0 {
            continue;
        }
        if rem < usize::from(comp.min) {
            return Err(AidcError::InvalidPayload(
                "AI value shorter than required component length".to_owned(),
            ));
        }

        let consume = if comp.min == comp.max {
            usize::from(comp.min)
        } else if idx + 1 == components.len() {
            rem
        } else {
            return Err(AidcError::InvalidPayload(
                "variable component must be final".to_owned(),
            ));
        };
        if consume < usize::from(comp.min) || consume > usize::from(comp.max) {
            return Err(AidcError::InvalidPayload(
                "AI component length out of bounds".to_owned(),
            ));
        }

        let segment = value.chars().skip(offset).take(consume).collect::<String>();
        if !segment.chars().all(|ch| validate_charset(ch, comp.charset)) {
            return Err(AidcError::InvalidPayload(
                "AI value has invalid character set".to_owned(),
            ));
        }
        offset += consume;
    }

    if offset != total_chars {
        return Err(AidcError::InvalidPayload(
            "AI value has trailing data beyond component spec".to_owned(),
        ));
    }
    Ok(())
}

fn validate_charset(ch: char, charset: AiCharset) -> bool {
    match charset {
        AiCharset::N => ch.is_ascii_digit(),
        AiCharset::X => ch.is_ascii() && !ch.is_ascii_control(),
        AiCharset::Y => matches!(
            ch,
            'A'..='Z'
                | '0'..='9'
                | ' '
                | '-'
                | '.'
                | '$'
                | '/'
                | '+'
                | '%'
        ),
        AiCharset::Z => ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'),
    }
}
