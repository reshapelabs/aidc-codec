pub(crate) fn fixed_value_length(ai: &str) -> Option<usize> {
    let bytes = ai.as_bytes();
    if bytes.len() < 2 || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
        return None;
    }
    let prefix = usize::from(bytes[0] - b'0') * 10 + usize::from(bytes[1] - b'0');
    const FIXED_BY_PREFIX: &[u8] = &[
        18, 14, 14, 14, 16, 0, 0, 0, 0, 0, 0, 6, 6, 6, 6, 6, 6, 6, 6, 6, 2, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 6, 6, 6, 6, 6, 6, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let n = FIXED_BY_PREFIX[prefix];
    if n == 0 { None } else { Some(n as usize) }
}

pub(crate) fn is_known_ai(ai: &str) -> bool {
    if ai.len() == 4 {
        if let Some(prefix) = ai.get(..3) {
            if matches!(prefix, "310" | "392") {
                return true;
            }
        }
    }
    matches!(
        ai,
        "00"
            | "01"
            | "02"
            | "10"
            | "11"
            | "12"
            | "13"
            | "15"
            | "16"
            | "17"
            | "20"
            | "21"
            | "22"
            | "37"
            | "89"
            | "98"
            | "99"
            | "235"
            | "253"
            | "254"
            | "255"
            | "401"
            | "402"
            | "414"
            | "417"
            | "8003"
            | "8004"
            | "8006"
            | "8010"
            | "8013"
            | "8017"
            | "8018"
            | "8019"
            | "8020"
    )
}

pub(crate) fn is_primary_ai(ai: &str) -> bool {
    matches!(
        ai,
        "00" | "01" | "253" | "255" | "401" | "402" | "414" | "417" | "8003" | "8004" | "8006"
            | "8010" | "8013" | "8017" | "8018" | "8020"
    )
}

pub(crate) fn is_dl_attribute_ai(ai: &str) -> bool {
    matches!(ai, "01" | "02" | "10" | "17" | "37" | "98" | "99" | "3103" | "3922")
}

pub(crate) fn ai_requires_fnc1(ai: &str) -> bool {
    !matches!(ai, "00" | "01" | "02" | "17" | "3103" | "414")
}
