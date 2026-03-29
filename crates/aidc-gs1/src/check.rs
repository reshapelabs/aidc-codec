use aidc_core::AidcError;

const THREE_WEIGHT_RESULTS: [u8; 10] = [0, 3, 6, 9, 12, 15, 18, 21, 24, 27];
const TWO_MINUS_WEIGHT_RESULTS: [u8; 10] = [0, 2, 4, 6, 8, 9, 1, 3, 5, 7];
const FIVE_PLUS_WEIGHT_RESULTS: [u8; 10] = [0, 5, 1, 6, 2, 7, 3, 8, 4, 9];
const FIVE_MINUS_WEIGHT_RESULTS: [u8; 10] = [0, 5, 9, 4, 8, 3, 7, 2, 6, 1];
const INVERSE_FIVE_MINUS_WEIGHT_RESULTS: [u8; 10] = [0, 9, 7, 5, 3, 1, 8, 6, 4, 2];

const CHECK_CHARACTER_WEIGHTS: [u16; 28] = [
    107, 103, 101, 97, 89, 83, 79, 73, 71, 67, 61, 59, 53, 47, 43, 41, 37, 31, 29, 23, 19, 17, 13,
    11, 7, 5, 3, 2,
];

const CHECK_CHARACTERS: [char; 32] = [
    '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L',
    'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

const AI82_CHARS: &str =
    "!\"%&'()*+,-./0123456789:;<=>?ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz";

fn numeric_char_at(s: &str, idx: usize) -> Result<u8, AidcError> {
    let b = s.as_bytes()[idx];
    if b.is_ascii_digit() {
        return Ok(b - b'0');
    }
    Err(AidcError::InvalidPayload(format!(
        "invalid character '{}' at position {}",
        s.as_bytes()[idx] as char,
        idx + 1
    )))
}

pub fn check_digit_sum(exchange_weights: bool, s: &str) -> Result<u32, AidcError> {
    let mut weight3 = (s.len() + usize::from(exchange_weights)).is_multiple_of(2);
    let mut sum = 0u32;
    for idx in 0..s.len() {
        let d = numeric_char_at(s, idx)?;
        weight3 = !weight3;
        sum += if weight3 {
            u32::from(THREE_WEIGHT_RESULTS[d as usize])
        } else {
            u32::from(d)
        };
    }
    Ok(sum)
}

pub fn check_digit(s: &str) -> Result<char, AidcError> {
    let sum = check_digit_sum(false, s)?;
    let digit = u8::try_from(9 - ((sum + 9) % 10)).expect("single digit value");
    Ok((b'0' + digit) as char)
}

pub fn has_valid_check_digit(s: &str) -> Result<bool, AidcError> {
    Ok(check_digit_sum(true, s)? % 10 == 0)
}

fn price_or_weight_sum(weights_results: &[&[u8; 10]], s: &str) -> Result<u32, AidcError> {
    let mut sum = 0u32;
    for (idx, weights) in weights_results.iter().enumerate() {
        let d = numeric_char_at(s, idx)?;
        sum += u32::from(weights[d as usize]);
    }
    Ok(sum)
}

pub fn price_or_weight_check_digit(s: &str) -> Result<char, AidcError> {
    let check = match s.len() {
        4 => {
            (price_or_weight_sum(
                &[
                    &TWO_MINUS_WEIGHT_RESULTS,
                    &TWO_MINUS_WEIGHT_RESULTS,
                    &THREE_WEIGHT_RESULTS,
                    &FIVE_MINUS_WEIGHT_RESULTS,
                ],
                s,
            )? * 3)
                % 10
        }
        5 => {
            let idx = (9
                - (price_or_weight_sum(
                    &[
                        &FIVE_PLUS_WEIGHT_RESULTS,
                        &TWO_MINUS_WEIGHT_RESULTS,
                        &FIVE_MINUS_WEIGHT_RESULTS,
                        &FIVE_PLUS_WEIGHT_RESULTS,
                        &TWO_MINUS_WEIGHT_RESULTS,
                    ],
                    s,
                )? + 9)
                    % 10) as usize;
            u32::from(INVERSE_FIVE_MINUS_WEIGHT_RESULTS[idx])
        }
        len => {
            return Err(AidcError::InvalidPayload(format!(
                "length {len} of price or weight must be 4 or 5"
            )));
        }
    };
    Ok((b'0' + u8::try_from(check).expect("single digit value")) as char)
}

pub fn is_valid_price_or_weight_check_digit(s: &str, check_digit: char) -> Result<bool, AidcError> {
    Ok(price_or_weight_check_digit(s)? == check_digit)
}

fn ai82_index(c: char, pos: usize) -> Result<usize, AidcError> {
    AI82_CHARS.find(c).ok_or_else(|| {
        AidcError::InvalidPayload(format!("invalid character '{c}' at position {}", pos + 1))
    })
}

pub fn check_character_pair(s: &str) -> Result<String, AidcError> {
    let weight_start = CHECK_CHARACTER_WEIGHTS
        .len()
        .checked_sub(s.len())
        .ok_or_else(|| {
            AidcError::InvalidPayload(format!(
                "length {} for check character pair must be <= 28",
                s.len()
            ))
        })?;

    let mut sum = 0u32;
    for (idx, c) in s.chars().enumerate() {
        let ci = ai82_index(c, idx)?;
        sum += (ci as u32) * u32::from(CHECK_CHARACTER_WEIGHTS[weight_start + idx]);
    }
    let v = sum % 1021;
    let rem = v % 32;
    Ok(format!(
        "{}{}",
        CHECK_CHARACTERS[((v - rem) / 32) as usize],
        CHECK_CHARACTERS[rem as usize]
    ))
}

pub fn has_valid_check_character_pair(s: &str) -> Result<bool, AidcError> {
    if s.len() < 2 {
        return Ok(false);
    }
    let idx = s.len() - 2;
    Ok(check_character_pair(&s[..idx])? == s[idx..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_digit_vectors() {
        let s = "1234567890";
        assert_eq!(
            check_digit_sum(false, s).expect("sum"),
            (1 + 3 + 5 + 7 + 9) + ((2 + 4 + 6 + 8) * 3)
        );
        assert_eq!(
            check_digit_sum(true, s).expect("sum"),
            (1 + 3 + 5 + 7 + 9) * 3 + (2 + 4 + 6 + 8)
        );
        let cd = check_digit(s).expect("check");
        assert_eq!(cd, '5');
        assert!(has_valid_check_digit(&format!("{s}{cd}")).expect("valid"));
        assert!(!has_valid_check_digit("12346678905").expect("valid"));
        assert!(check_digit("123456789O").is_err());
    }

    #[test]
    fn price_or_weight_check_digit_vectors() {
        let c4 = price_or_weight_check_digit("0123").expect("digit");
        assert!(is_valid_price_or_weight_check_digit("0123", c4).expect("valid"));
        let c5 = price_or_weight_check_digit("12345").expect("digit");
        assert!(is_valid_price_or_weight_check_digit("12345", c5).expect("valid"));
        assert!(price_or_weight_check_digit("123").is_err());
        assert!(price_or_weight_check_digit("123456").is_err());
        assert!(price_or_weight_check_digit("l2345").is_err());
    }

    #[test]
    fn check_character_pair_vectors() {
        assert_eq!(check_character_pair("95212349521234").expect("pair"), "R9");
        assert_eq!(
            check_character_pair("9521234ABCDEFabcdef").expect("pair"),
            "8T"
        );
        assert_eq!(
            check_character_pair("!\"%&'()*+,-./0123456789:;<=>").expect("pair"),
            "TH"
        );
        assert_eq!(
            check_character_pair("?ABCDEFGHIJKLMNOPQRSTUVWXYZ_").expect("pair"),
            "EP"
        );
        assert_eq!(
            check_character_pair("abcdefghijklmnopqrstuvwxyz").expect("pair"),
            "5A"
        );
        assert!(has_valid_check_character_pair("95212349521234R9").expect("valid"));
        assert!(has_valid_check_character_pair("9521234ABCDEFabcdef8T").expect("valid"));
        assert!(has_valid_check_character_pair("!\"%&'()*+,-./0123456789:;<=>TH").expect("valid"));
        assert!(has_valid_check_character_pair("?ABCDEFGHIJKLMNOPQRSTUVWXYZ_EP").expect("valid"));
        assert!(has_valid_check_character_pair("abcdefghijklmnopqrstuvwxyz5A").expect("valid"));
        assert!(!has_valid_check_character_pair("abcdefghijklmnopqrstuvwxyz5B").expect("valid"));
        assert!(check_character_pair("abcdefghijklmnopqrstuvwxyz~").is_err());
    }
}
