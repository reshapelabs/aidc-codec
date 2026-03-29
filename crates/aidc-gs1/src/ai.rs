use aidc_core::AidcError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiCharset {
    N,
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiDateRule {
    None,
    YymmddStrict,
    YymmddAllowZeroDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiTimeRule {
    None,
    Hhmi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AiComponent {
    pub charset: AiCharset,
    pub min: u8,
    pub max: u8,
    pub optional: bool,
    pub mod10_check: bool,
    pub date_rule: AiDateRule,
    pub time_rule: AiTimeRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AiReqGroup {
    pub all_of: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AiReqClause {
    pub any_of: &'static [AiReqGroup],
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
    pub req_rules: &'static [AiReqClause],
    pub ex_rules: &'static [&'static str],
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
        return Err(AidcError::InvalidPayload(
            "AI value must not be empty".to_owned(),
        ));
    }
    if value.len() > 90 {
        return Err(AidcError::InvalidPayload("AI value too long".to_owned()));
    }

    let Some(meta) = lookup_ai(ai) else {
        return Ok(());
    };
    validate_components(meta.components, value)
}

pub(crate) fn validate_message_rules<'a>(
    ais: impl IntoIterator<Item = &'a str>,
) -> Result<(), AidcError> {
    let present = ais.into_iter().map(str::to_owned).collect::<Vec<_>>();
    for ai in &present {
        let Some(meta) = lookup_ai(ai) else {
            continue;
        };
        validate_required_rules(ai, meta.req_rules, &present)?;
        validate_exclusive_rules(ai, meta.ex_rules, &present)?;
    }
    Ok(())
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
        if matches!(comp.charset, AiCharset::Z) {
            validate_base64url_segment(&segment)?;
        }
        validate_component_semantics(comp, &segment)?;
        offset += consume;
    }

    if offset != total_chars {
        return Err(AidcError::InvalidPayload(
            "AI value has trailing data beyond component spec".to_owned(),
        ));
    }
    Ok(())
}

fn validate_component_semantics(comp: &AiComponent, segment: &str) -> Result<(), AidcError> {
    if comp.mod10_check && !valid_mod10(segment) {
        return Err(AidcError::InvalidPayload(
            "AI value has invalid check digit".to_owned(),
        ));
    }

    match comp.date_rule {
        AiDateRule::None => {}
        AiDateRule::YymmddStrict => validate_yymmdd(segment, false)?,
        AiDateRule::YymmddAllowZeroDay => validate_yymmdd(segment, true)?,
    }

    if matches!(comp.time_rule, AiTimeRule::Hhmi) {
        validate_hhmi(segment)?;
    }
    Ok(())
}

fn validate_charset(ch: char, charset: AiCharset) -> bool {
    match charset {
        AiCharset::N => ch.is_ascii_digit(),
        AiCharset::X => is_ai82_char(ch),
        AiCharset::Y => is_ai39_char(ch),
        AiCharset::Z => ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '='),
    }
}

fn is_ai82_char(ch: char) -> bool {
    matches!(
        ch,
        '!' | '"' | '%' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | '-' | '.' | '/'
            | '0'..='9'
            | ':'
            | ';'
            | '<'
            | '='
            | '>'
            | '?'
            | 'A'..='Z'
            | '_'
            | 'a'..='z'
    )
}

fn is_ai39_char(ch: char) -> bool {
    matches!(ch, '#' | '-' | '/' | '0'..='9' | 'A'..='Z')
}

fn valid_mod10(value: &str) -> bool {
    if value.len() < 2 || !value.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let mut sum = 0u32;
    for (idx, ch) in value[..value.len() - 1].chars().rev().enumerate() {
        let d = u32::from((ch as u8) - b'0');
        sum += if idx % 2 == 0 { 3 * d } else { d };
    }
    let check = (10 - (sum % 10)) % 10;
    value.as_bytes()[value.len() - 1] == b'0' + (check as u8)
}

fn validate_yymmdd(value: &str, allow_zero_day: bool) -> Result<(), AidcError> {
    if value.len() != 6 || !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(AidcError::InvalidPayload(
            "AI value has invalid date format".to_owned(),
        ));
    }
    let month = ((value.as_bytes()[2] - b'0') as u16) * 10 + u16::from(value.as_bytes()[3] - b'0');
    let day = ((value.as_bytes()[4] - b'0') as u16) * 10 + u16::from(value.as_bytes()[5] - b'0');

    if !(1..=12).contains(&month) {
        return Err(AidcError::InvalidPayload(
            "AI value has invalid month".to_owned(),
        ));
    }

    if day == 0 {
        if allow_zero_day {
            return Ok(());
        }
        return Err(AidcError::InvalidPayload(
            "AI value has invalid day".to_owned(),
        ));
    }

    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 29,
        _ => 0,
    };
    if day > max_day {
        return Err(AidcError::InvalidPayload(
            "AI value has invalid day".to_owned(),
        ));
    }
    Ok(())
}

fn validate_hhmi(value: &str) -> Result<(), AidcError> {
    if value.len() != 4 || !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(AidcError::InvalidPayload(
            "AI value has invalid time format".to_owned(),
        ));
    }
    let hh = ((value.as_bytes()[0] - b'0') as u16) * 10 + u16::from(value.as_bytes()[1] - b'0');
    let mm = ((value.as_bytes()[2] - b'0') as u16) * 10 + u16::from(value.as_bytes()[3] - b'0');
    if hh > 23 || mm > 59 {
        return Err(AidcError::InvalidPayload(
            "AI value has invalid time value".to_owned(),
        ));
    }
    Ok(())
}

fn validate_base64url_segment(value: &str) -> Result<(), AidcError> {
    if !value.len().is_multiple_of(4) {
        return Err(AidcError::InvalidPayload(
            "AI value has invalid base64url length".to_owned(),
        ));
    }
    if let Some(eq_pos) = value.find('=') {
        let pad = &value[eq_pos..];
        if pad != "=" && pad != "==" {
            return Err(AidcError::InvalidPayload(
                "AI value has invalid base64url padding".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_required_rules(
    ai: &str,
    req: &[AiReqClause],
    present: &[String],
) -> Result<(), AidcError> {
    for clause in req {
        let satisfied = clause.any_of.iter().any(|group| {
            group
                .all_of
                .iter()
                .all(|pattern| present.iter().any(|x| matches_ai_pattern(x, pattern)))
        });
        if !satisfied {
            return Err(AidcError::InvalidPayload(format!(
                "AI {ai} missing required association"
            )));
        }
    }
    Ok(())
}

fn validate_exclusive_rules(ai: &str, ex: &[&str], present: &[String]) -> Result<(), AidcError> {
    for pattern in ex {
        let conflict = present
            .iter()
            .any(|other| other != ai && matches_ai_pattern(other, pattern));
        if conflict {
            return Err(AidcError::InvalidPayload(format!(
                "AI {ai} has forbidden association ({pattern})"
            )));
        }
    }
    Ok(())
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
    use super::{validate_ai_value, validate_message_rules};

    #[test]
    fn validates_ai_01_numeric_fixed_length() {
        assert!(validate_ai_value("01", "09520123456788").is_ok());
        assert!(validate_ai_value("01", "0952012345678").is_err());
        assert!(validate_ai_value("01", "0952012345678A").is_err());
    }

    #[test]
    fn validates_ai_10_variable_x_charset() {
        assert!(validate_ai_value("10", "ABC123-._/").is_ok());
        assert!(validate_ai_value("10", "ABC\u{0001}123").is_err());
    }

    #[test]
    fn validates_ai82_boundary_vectors() {
        assert!(validate_ai_value("91", "!\"%&'()*+,-./0123456789:;<=>?ABCXYZ_abcxyz").is_ok());
        for invalid in [" ", "#", "@", "[", "\\", "]", "^", "`", "{", "|", "}", "~"] {
            assert!(
                validate_ai_value("91", invalid).is_err(),
                "expected invalid AI82 char {invalid:?}"
            );
        }
    }

    #[test]
    fn validates_ai39_boundary_vectors() {
        assert!(validate_ai_value("8010", "#-/0123456789ABCDEFGHIJKLMNOP").is_ok());
        for invalid in ["a", "_", ".", "$", "+", "%", " "] {
            assert!(
                validate_ai_value("8010", invalid).is_err(),
                "expected invalid AI39 char {invalid:?}"
            );
        }
    }

    #[test]
    fn validates_ai_17_fixed_numeric_date_shape() {
        assert!(validate_ai_value("17", "251231").is_ok());
        assert!(validate_ai_value("17", "250200").is_ok());
        assert!(validate_ai_value("17", "25123").is_err());
        assert!(validate_ai_value("17", "25AA31").is_err());
        assert!(validate_ai_value("17", "251332").is_err());
        assert!(validate_ai_value("17", "250231").is_err());
    }

    #[test]
    fn validates_ai_253_multipart_constraints() {
        assert!(validate_ai_value("253", "9520123456788").is_ok());
        assert!(validate_ai_value("253", "9520123456788ABC").is_ok());
        assert!(validate_ai_value("253", "9520123456787").is_err());
        assert!(validate_ai_value("253", "1234567890123ABCDEFGHIJKLMNOPQR").is_err());
    }

    #[test]
    fn validates_ai_8030_base64url_charset() {
        assert!(validate_ai_value("8030", "AbC123-_").is_ok());
        assert!(validate_ai_value(
            "8030",
            "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyzABC="
        )
        .is_ok());
        assert!(validate_ai_value(
            "8030",
            "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyzAB=="
        )
        .is_ok());
        assert!(validate_ai_value("8030", "AbC/123").is_err());
        assert!(validate_ai_value(
            "8030",
            "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxy"
        )
        .is_err());
        assert!(validate_ai_value(
            "8030",
            "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwx"
        )
        .is_err());
        assert!(validate_ai_value(
            "8030",
            "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvw"
        )
        .is_err());
        assert!(validate_ai_value(
            "8030",
            "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyzA==="
        )
        .is_err());
        assert!(validate_ai_value(
            "8030",
            "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz===="
        )
        .is_err());
        assert!(validate_ai_value(
            "8030",
            "=-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxy"
        )
        .is_err());
        assert!(validate_ai_value(
            "8030",
            "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ=_abcdefghijklmnopqrstuvwxy"
        )
        .is_err());
    }

    #[test]
    fn validates_mod10_for_gtin_and_gln() {
        assert!(validate_ai_value("01", "09520123456788").is_ok());
        assert!(validate_ai_value("01", "09520123456789").is_err());
        assert!(validate_ai_value("414", "9520123456788").is_ok());
        assert!(validate_ai_value("414", "9520123456787").is_err());
    }

    #[test]
    fn validates_hhmi_time_component() {
        assert!(validate_ai_value("7003", "2601012359").is_ok());
        assert!(validate_ai_value("7003", "2601012460").is_err());
    }

    #[test]
    fn validates_required_ai_associations() {
        assert!(validate_message_rules(["11", "01"]).is_ok());
        assert!(validate_message_rules(["11"]).is_err());
        assert!(validate_message_rules(["250", "21", "01"]).is_ok());
        assert!(validate_message_rules(["250", "21"]).is_err());
    }

    #[test]
    fn validates_exclusive_ai_associations() {
        assert!(validate_message_rules(["01", "255"]).is_err());
        assert!(validate_message_rules(["3940", "255"]).is_ok());
        assert!(validate_message_rules(["3940", "3941", "255"]).is_err());
    }

    #[test]
    fn validates_required_pattern_associations() {
        assert!(validate_message_rules(["3920", "01", "3102"]).is_ok());
        assert!(validate_message_rules(["3920", "01"]).is_err());
    }
}
