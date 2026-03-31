use aidc_core::AidcError;
use std::collections::HashSet;

use crate::ai::{
    ai_requires_fnc1, fixed_value_length, is_dl_attribute_ai, is_known_ai, is_primary_ai,
    validate_ai_value as validate_ai_value_by_meta,
};
use crate::model::SymbologyId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanDataOutcome {
    pub sym_name: &'static str,
    pub data_str: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DlParseOptions {
    pub permit_convenience_alphas: bool,
    pub permit_zero_suppressed_gtin: bool,
    pub permit_unknown_ais: bool,
    pub validate_unknown_ai_not_dl_attr: bool,
}

pub fn parse_bracketed_ai(input: &str) -> Result<String, AidcError> {
    if !input.starts_with('(') {
        return Err(AidcError::InvalidInput(
            "AI data must start with bracketed AI".to_owned(),
        ));
    }

    let mut idx = 0usize;
    let bytes = input.as_bytes();
    let mut out = String::from("^");
    let mut first = true;
    let mut prev_requires_fnc1 = false;

    while idx < bytes.len() {
        if bytes[idx] != b'(' {
            return Err(AidcError::InvalidInput(
                "expected AI opening bracket".to_owned(),
            ));
        }
        idx += 1;

        let ai_start = idx;
        while idx < bytes.len() && bytes[idx] != b')' {
            if !bytes[idx].is_ascii_digit() {
                return Err(AidcError::InvalidInput("AI must be numeric".to_owned()));
            }
            idx += 1;
        }
        if idx >= bytes.len() || bytes[idx] != b')' {
            return Err(AidcError::InvalidInput(
                "AI must terminate with ')'".to_owned(),
            ));
        }

        let ai = &input[ai_start..idx];
        if ai.len() < 2 || ai.len() > 4 {
            return Err(AidcError::InvalidInput("AI length must be 2..4".to_owned()));
        }
        idx += 1;

        let mut value = String::new();
        while idx < bytes.len() {
            match bytes[idx] {
                b'\\' => {
                    if idx + 1 < bytes.len() && bytes[idx + 1] == b'(' {
                        value.push('(');
                        idx += 2;
                    } else {
                        value.push('\\');
                        idx += 1;
                    }
                }
                b'(' => break,
                b'^' => {
                    return Err(AidcError::InvalidInput(
                        "caret is reserved for FNC1 encoding".to_owned(),
                    ));
                }
                c => {
                    value.push(c as char);
                    idx += 1;
                }
            }
        }

        if value.is_empty() {
            return Err(AidcError::InvalidInput(
                "AI value must not be empty".to_owned(),
            ));
        }
        if value.len() > 90 {
            return Err(AidcError::InvalidInput("AI value too long".to_owned()));
        }

        let fixed_len = fixed_value_length(ai);
        if let Some(n) = fixed_len {
            if value.len() != n {
                return Err(AidcError::InvalidInput(
                    "fixed-length AI has invalid length".to_owned(),
                ));
            }
        }

        if !first && prev_requires_fnc1 {
            out.push('^');
        }
        first = false;
        out.push_str(ai);
        out.push_str(&value);

        prev_requires_fnc1 = fixed_len.is_none();
    }

    if out == "^" {
        return Err(AidcError::InvalidInput("empty AI payload".to_owned()));
    }

    Ok(out)
}

pub fn process_scan_data(scan_data: &str) -> Result<ScanDataOutcome, AidcError> {
    if scan_data.len() < 3 || !scan_data.starts_with(']') {
        return Err(AidcError::InvalidInput(
            "missing symbology identifier".to_owned(),
        ));
    }

    let (id, payload) = scan_data.split_at(3);
    let sid = SymbologyId::parse(id);

    match sid {
        SymbologyId::Q3
        | SymbologyId::D2
        | SymbologyId::J1
        | SymbologyId::LowerE0
        | SymbologyId::C1 => {
            if payload.is_empty() {
                return Err(AidcError::InvalidInput("empty GS1 scan payload".to_owned()));
            }
            if payload.contains('^') {
                return Err(AidcError::InvalidInput(
                    "raw '^' is not valid in scan payload".to_owned(),
                ));
            }
            let mut data = String::from("^");
            for ch in payload.chars() {
                if ch == '\u{001d}' {
                    data.push('^');
                } else {
                    data.push(ch);
                }
            }
            Ok(ScanDataOutcome {
                sym_name: match sid {
                    SymbologyId::Q3 => "QR",
                    SymbologyId::D2 => "DM",
                    SymbologyId::J1 => "DotCode",
                    SymbologyId::LowerE0 => "DataBarExpanded",
                    SymbologyId::C1 => "GS1_128_CCA",
                    _ => unreachable!(),
                },
                data_str: data,
            })
        }
        SymbologyId::Q1 | SymbologyId::D1 | SymbologyId::J0 => {
            if looks_like_uri(payload) && !payload.contains("/01/") {
                return Err(AidcError::InvalidInput(
                    "digital link URI without primary key AI".to_owned(),
                ));
            }
            Ok(ScanDataOutcome {
                sym_name: match sid {
                    SymbologyId::Q1 => "QR",
                    SymbologyId::D1 => "DM",
                    SymbologyId::J0 => "DotCode",
                    _ => unreachable!(),
                },
                data_str: escape_leading_caret(payload),
            })
        }
        SymbologyId::Unknown(_) => Err(AidcError::UnsupportedSymbologyId(id.to_owned())),
        SymbologyId::E0 => process_ean(payload, "EAN13", 13),
        SymbologyId::E4 => process_ean(payload, "EAN8", 8),
        _ => Err(AidcError::UnsupportedSymbologyId(id.to_owned())),
    }
}

fn process_ean(
    payload: &str,
    sym_name: &'static str,
    digits: usize,
) -> Result<ScanDataOutcome, AidcError> {
    let (primary, composite) = if let Some(pos) = payload.find("|]e0") {
        (&payload[..pos], Some(&payload[pos + 4..]))
    } else {
        (payload, None)
    };

    if primary.len() != digits || !primary.as_bytes().iter().all(u8::is_ascii_digit) {
        return Err(AidcError::InvalidInput(
            "invalid EAN primary data".to_owned(),
        ));
    }
    if !valid_mod10(primary) {
        return Err(AidcError::InvalidInput(
            "invalid EAN check digit".to_owned(),
        ));
    }

    let mut out = primary.to_owned();
    if let Some(cc) = composite {
        if cc.is_empty() {
            return Err(AidcError::InvalidInput(
                "empty composite payload".to_owned(),
            ));
        }
        let mut cc_data = String::from("^");
        for ch in cc.chars() {
            if ch == '\u{001d}' {
                cc_data.push('^');
            } else {
                cc_data.push(ch);
            }
        }
        out.push('|');
        out.push_str(&cc_data);
    }

    Ok(ScanDataOutcome {
        sym_name,
        data_str: out,
    })
}

fn escape_leading_caret(payload: &str) -> String {
    let slash_count = payload
        .as_bytes()
        .iter()
        .take_while(|&&b| b == b'\\')
        .count();
    if payload.as_bytes().get(slash_count) == Some(&b'^') {
        let mut out = String::with_capacity(payload.len() + 1);
        out.push('\\');
        out.push_str(payload);
        out
    } else {
        payload.to_owned()
    }
}

fn looks_like_uri(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("HTTP://")
        || s.starts_with("HTTPS://")
        || s.starts_with("HtTps://")
}

fn valid_mod10(digits: &str) -> bool {
    if !digits.as_bytes().iter().all(u8::is_ascii_digit) || digits.len() < 2 {
        return false;
    }

    let vals: Vec<u32> = digits.bytes().map(|b| u32::from(b - b'0')).collect();
    let mut sum = 0u32;
    let mut weight = if vals.len().is_multiple_of(2) { 3 } else { 1 };
    for n in &vals[..vals.len() - 1] {
        sum += *n * weight;
        weight = 4 - weight;
    }
    let check = (10 - (sum % 10)) % 10;
    check == vals[vals.len() - 1]
}

pub fn parse_dl_uri(input: &str, options: DlParseOptions) -> Result<String, AidcError> {
    if input.is_empty() {
        return Err(AidcError::InvalidInput("empty URI".to_owned()));
    }

    if input
        .chars()
        .any(|c| matches!(c, '<' | '>' | '{' | '}' | '\\' | '^' | '`'))
    {
        return Err(AidcError::InvalidInput("illegal URI character".to_owned()));
    }

    let rest = if let Some(r) = input.strip_prefix("http://") {
        r
    } else if let Some(r) = input.strip_prefix("https://") {
        r
    } else if let Some(r) = input.strip_prefix("HTTP://") {
        r
    } else if let Some(r) = input.strip_prefix("HTTPS://") {
        r
    } else {
        return Err(AidcError::InvalidInput("unsupported URI scheme".to_owned()));
    };

    let rest = rest.split('#').next().unwrap_or(rest);
    let (before_query, query) = rest.split_once('?').unwrap_or((rest, ""));
    let slash = before_query
        .find('/')
        .ok_or_else(|| AidcError::InvalidInput("URI missing path information".to_owned()))?;
    let authority = &before_query[..slash];
    let path = &before_query[slash..];
    if authority.is_empty() || path.is_empty() {
        return Err(AidcError::InvalidInput(
            "URI missing authority or path".to_owned(),
        ));
    }
    let is_gs1_resolver = authority.eq_ignore_ascii_case("id.gs1.org");
    if authority.chars().any(|c| {
        matches!(
            c,
            '_' | '~'
                | '?'
                | '#'
                | '@'
                | '!'
                | '$'
                | '&'
                | '\''
                | '('
                | ')'
                | '*'
                | '+'
                | ','
                | ';'
                | '='
                | '%'
        )
    }) {
        return Err(AidcError::InvalidInput(
            "invalid character in authority".to_owned(),
        ));
    }

    let raw_segments: Vec<&str> = path.split('/').skip(1).collect();
    if raw_segments.is_empty() || raw_segments.last() == Some(&"") {
        return Err(AidcError::InvalidInput("invalid path segments".to_owned()));
    }

    let mut path_segments = Vec::with_capacity(raw_segments.len());
    for s in raw_segments {
        path_segments.push(uri_unescape(s, false)?);
    }

    let start = find_primary_start(&path_segments, options)?;
    let mut elements = Vec::<(String, String)>::new();
    let mut used_ais = HashSet::<String>::new();

    let mut i = start;
    while i < path_segments.len() {
        if i + 1 >= path_segments.len() {
            return Err(AidcError::InvalidInput("path key missing value".to_owned()));
        }
        let key_raw = &path_segments[i];
        let mut ai = normalize_ai(key_raw, options)?;
        let mut value = path_segments[i + 1].clone();
        if value.is_empty() {
            return Err(AidcError::InvalidInput(
                "path AI value must not be empty".to_owned(),
            ));
        }
        if ai == "01"
            && start > 0
            && !options.permit_zero_suppressed_gtin
            && value.chars().all(|c| c.is_ascii_digit())
            && matches!(value.len(), 13 | 12 | 8)
        {
            value = format!("{:0>14}", value);
        }
        validate_ai_value(&mut ai, &mut value, options)?;
        if !used_ais.insert(ai.clone()) {
            return Err(AidcError::InvalidInput("repeated AI in path".to_owned()));
        }
        if elements.is_empty() {
            if !is_primary_ai(&ai) {
                return Err(AidcError::InvalidInput(
                    "path must begin with primary key AI".to_owned(),
                ));
            }
        } else {
            let p = &elements[0].0;
            if !is_valid_path_qualifier(p, &ai) {
                return Err(AidcError::InvalidInput(
                    "invalid path qualifier for primary AI".to_owned(),
                ));
            }
        }
        elements.push((ai, value));
        i += 2;
    }

    for part in query.split('&').filter(|p| !p.is_empty()) {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        if k.is_empty() {
            continue;
        }
        let key = uri_unescape(k, true)?;
        let mut val = uri_unescape(v, true)?;
        if !key.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if val.is_empty() {
            return Err(AidcError::InvalidInput(
                "query AI value must not be empty".to_owned(),
            ));
        }

        if !is_known_ai(&key) {
            if !options.permit_unknown_ais || options.validate_unknown_ai_not_dl_attr {
                return Err(AidcError::InvalidInput(
                    "unknown numeric query AI is not permitted".to_owned(),
                ));
            }
        } else if options.validate_unknown_ai_not_dl_attr && !is_dl_attribute_ai(&key) {
            return Err(AidcError::InvalidInput(
                "numeric query AI is not a DL attribute".to_owned(),
            ));
        }
        if !used_ais.insert(key.clone()) {
            return Err(AidcError::InvalidInput(
                "repeated AI in query/path".to_owned(),
            ));
        }

        let mut ai = key;
        if !is_gs1_resolver && ai == "10" {
            return Err(AidcError::InvalidInput(
                "AI not allowed in query for this resolver".to_owned(),
            ));
        }
        if ai == "01" && val.chars().all(|c| c.is_ascii_digit()) && matches!(val.len(), 13 | 12 | 8)
        {
            val = format!("{:0>14}", val);
        }
        validate_ai_value(&mut ai, &mut val, options)?;
        elements.push((ai, val));
    }

    if elements.is_empty() || !is_primary_ai(&elements[0].0) {
        return Err(AidcError::InvalidInput(
            "no primary key AI in URI path".to_owned(),
        ));
    }

    Ok(to_internal_ai_string(&elements))
}

fn uri_unescape(s: &str, query: bool) -> Result<String, AidcError> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' if query => out.push(b' '),
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(AidcError::InvalidInput("bad percent-escape".to_owned()));
                }
                let h1 = bytes[i + 1] as char;
                let h2 = bytes[i + 2] as char;
                let hex = format!("{h1}{h2}");
                let v = u8::from_str_radix(&hex, 16)
                    .map_err(|_| AidcError::InvalidInput("bad percent-escape".to_owned()))?;
                if v == 0 {
                    return Err(AidcError::InvalidInput("NUL byte not allowed".to_owned()));
                }
                out.push(v);
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

fn find_primary_start(segments: &[String], options: DlParseOptions) -> Result<usize, AidcError> {
    if segments.len() < 2 {
        return Err(AidcError::InvalidInput(
            "path does not contain AI/value pair".to_owned(),
        ));
    }
    for i in (0..segments.len().saturating_sub(1)).rev() {
        if !(segments.len() - i).is_multiple_of(2) {
            continue;
        }
        let ai = match normalize_ai(&segments[i], options) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if is_primary_ai(&ai) {
            return Ok(i);
        }
    }
    Err(AidcError::InvalidInput(
        "no primary key AI in path".to_owned(),
    ))
}

fn normalize_ai(key: &str, options: DlParseOptions) -> Result<String, AidcError> {
    if key.chars().all(|c| c.is_ascii_digit()) && (2..=4).contains(&key.len()) {
        return Ok(key.to_owned());
    }
    if options.permit_convenience_alphas {
        let mapped = match key {
            "gtin" => Some("01"),
            "sscc" => Some("00"),
            "ser" => Some("21"),
            _ => None,
        };
        if let Some(ai) = mapped {
            return Ok(ai.to_owned());
        }
    }
    Err(AidcError::InvalidInput("unrecognized AI key".to_owned()))
}

fn is_valid_path_qualifier(primary_ai: &str, ai: &str) -> bool {
    match primary_ai {
        "01" => matches!(ai, "22" | "10" | "21" | "235"),
        "414" => ai == "254",
        "8018" => ai == "8019",
        _ => false,
    }
}

fn validate_ai_value(
    ai: &mut String,
    value: &mut String,
    options: DlParseOptions,
) -> Result<(), AidcError> {
    if *ai == "01"
        && options.permit_zero_suppressed_gtin
        && value.chars().all(|c| c.is_ascii_digit())
        && matches!(value.len(), 13 | 12 | 8)
    {
        *value = format!("{:0>14}", value);
    }
    validate_ai_value_by_meta(ai, value).map_err(|err| match err {
        AidcError::InvalidPayload(msg) => AidcError::InvalidInput(msg),
        other => other,
    })
}

fn to_internal_ai_string(elements: &[(String, String)]) -> String {
    let mut out = String::from("^");
    let mut prev_var = false;
    for (idx, (ai, value)) in elements.iter().enumerate() {
        if idx > 0 && prev_var {
            out.push('^');
        }
        out.push_str(ai);
        out.push_str(value);
        prev_var = ai_requires_fnc1(ai);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{parse_dl_uri, DlParseOptions};

    #[test]
    fn dl_query_duplicate_ai_is_rejected() {
        let err = parse_dl_uri(
            "https://id.gs1.org/01/09520123456788?99=ABC&99=XYZ",
            DlParseOptions::default(),
        )
        .expect_err("duplicate query AI should fail");
        assert!(err.to_string().contains("repeated AI in query/path"));
    }

    #[test]
    fn dl_query_path_duplicate_ai_is_rejected() {
        let err = parse_dl_uri(
            "https://id.gs1.org/01/09520123456788/10/ABC?10=XYZ",
            DlParseOptions::default(),
        )
        .expect_err("duplicate path/query AI should fail");
        assert!(err.to_string().contains("repeated AI in query/path"));
    }

    #[test]
    fn dl_query_order_is_preserved_in_internal_representation() {
        let internal = parse_dl_uri(
            "https://id.gs1.org/01/09520123456788?17=201225&3103=000195&3922=0299",
            DlParseOptions::default(),
        )
        .expect("parse should succeed");
        assert_eq!(internal, "^010952012345678817201225310300019539220299");
    }

    #[test]
    fn dl_path_then_query_variable_fields_insert_separator() {
        let internal = parse_dl_uri(
            "https://id.gs1.org/01/09520123456788/10/ABC123?99=XYZ",
            DlParseOptions::default(),
        )
        .expect("parse should succeed");
        assert_eq!(internal, "^010952012345678810ABC123^99XYZ");
    }

    #[test]
    fn dl_unknown_numeric_query_ai_rejected_when_not_permitted() {
        let err = parse_dl_uri(
            "https://example.com/01/09520123456788?89=ABC123",
            DlParseOptions::default(),
        )
        .expect_err("unknown numeric query AI should fail by default");
        assert!(err
            .to_string()
            .contains("unknown numeric query AI is not permitted"));
    }

    #[test]
    fn dl_unknown_numeric_query_ai_allowed_when_permitted() {
        let internal = parse_dl_uri(
            "https://example.com/01/09520123456788?89=ABC123",
            DlParseOptions {
                permit_unknown_ais: true,
                validate_unknown_ai_not_dl_attr: false,
                ..DlParseOptions::default()
            },
        )
        .expect("unknown numeric query AI should parse when permitted");
        assert_eq!(internal, "^010952012345678889ABC123");
    }

    #[test]
    fn dl_convenience_alpha_key_rejected_when_option_disabled() {
        let err = parse_dl_uri(
            "https://example.com/gtin/12312312312333",
            DlParseOptions::default(),
        )
        .expect_err("convenience alpha key should fail by default");
        assert!(err.to_string().contains("no primary key AI in path"));
    }

    #[test]
    fn dl_convenience_alpha_key_mapping_enabled_by_option() {
        let internal = parse_dl_uri(
            "https://example.com/gtin/12312312312333/ser/ABC123",
            DlParseOptions {
                permit_convenience_alphas: true,
                ..DlParseOptions::default()
            },
        )
        .expect("convenience alpha keys should map when enabled");
        assert_eq!(internal, "^011231231231233321ABC123");
    }
}
