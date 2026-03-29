use aidc_core::AidcError;

use crate::check::{
    check_digit, has_valid_check_digit, is_valid_price_or_weight_check_digit,
    price_or_weight_check_digit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RcnReference {
    pub item_reference: u32,
    pub price_or_weight: u32,
}

pub struct VariableMeasure;

impl VariableMeasure {
    pub fn parse_rcn(format: &str, rcn: &str) -> Result<RcnReference, AidcError> {
        if format.len() != rcn.len() {
            return Err(AidcError::InvalidPayload(
                "RCN length must match format length".to_owned(),
            ));
        }
        validate_format_shape(format)?;
        validate_prefix(format, rcn)?;

        let mut item = String::new();
        let mut price = String::new();
        let mut price_check: Option<char> = None;
        let mut in_item = false;
        let mut in_price = false;
        let mut seen_item = false;
        let mut seen_price = false;

        for (idx, (f, c)) in format.chars().zip(rcn.chars()).enumerate() {
            if idx == 0 || (format.len() == 13 && idx == 1) || idx + 1 == format.len() {
                continue;
            }
            match f {
                'I' => {
                    if !in_item {
                        if seen_item {
                            return Err(AidcError::InvalidPayload(
                                "invalid variable measure RCN format".to_owned(),
                            ));
                        }
                        seen_item = true;
                        in_item = true;
                        in_price = false;
                    }
                    item.push(c);
                }
                'P' => {
                    if !in_price {
                        if seen_price {
                            return Err(AidcError::InvalidPayload(
                                "invalid variable measure RCN format".to_owned(),
                            ));
                        }
                        seen_price = true;
                        in_price = true;
                        in_item = false;
                    }
                    price.push(c);
                }
                'V' => {
                    if price_check.is_some() {
                        return Err(AidcError::InvalidPayload(
                            "invalid variable measure RCN format".to_owned(),
                        ));
                    }
                    in_item = false;
                    in_price = false;
                    price_check = Some(c);
                }
                _ => {
                    return Err(AidcError::InvalidPayload(
                        "invalid variable measure RCN format".to_owned(),
                    ));
                }
            }
        }

        if item.is_empty() || price.is_empty() {
            return Err(AidcError::InvalidPayload(
                "invalid variable measure RCN format".to_owned(),
            ));
        }
        if !item.chars().all(|c| c.is_ascii_digit()) || !price.chars().all(|c| c.is_ascii_digit()) {
            return Err(AidcError::InvalidPayload(
                "invalid variable measure RCN format".to_owned(),
            ));
        }
        if let Some(pc) = price_check {
            if !is_valid_price_or_weight_check_digit(&price, pc)? {
                return Err(AidcError::InvalidPayload(
                    "invalid variable measure price or weight".to_owned(),
                ));
            }
        }
        if !has_valid_check_digit(rcn)? {
            return Err(AidcError::InvalidPayload("invalid check digit".to_owned()));
        }
        Ok(RcnReference {
            item_reference: item.parse::<u32>().expect("digits only"),
            price_or_weight: price.parse::<u32>().expect("digits only"),
        })
    }

    pub fn create_rcn(
        format: &str,
        item_reference: u32,
        price_or_weight: u32,
    ) -> Result<String, AidcError> {
        validate_format_shape(format)?;
        let item_len = format.chars().filter(|c| *c == 'I').count();
        let price_len = format.chars().filter(|c| *c == 'P').count();
        let has_price_check = format.contains('V');

        if item_len == 0 || price_len == 0 {
            return Err(AidcError::InvalidPayload(
                "invalid variable measure RCN format".to_owned(),
            ));
        }

        let item = pad_numeric(item_reference, item_len)?;
        let price = pad_numeric(price_or_weight, price_len)?;

        let mut out = String::new();
        let mut i_i = 0usize;
        let mut p_i = 0usize;
        for (idx, f) in format.chars().enumerate() {
            if idx + 1 == format.len() {
                break;
            }
            match f {
                '2' | '0'..='9' if idx == 0 || (format.len() == 13 && idx == 1) => out.push(f),
                'I' => {
                    out.push(item.as_bytes()[i_i] as char);
                    i_i += 1;
                }
                'P' => {
                    out.push(price.as_bytes()[p_i] as char);
                    p_i += 1;
                }
                'V' => out.push(price_or_weight_check_digit(&price)?),
                _ => {
                    return Err(AidcError::InvalidPayload(
                        "invalid variable measure RCN format".to_owned(),
                    ));
                }
            }
        }
        if has_price_check && out.is_empty() {
            return Err(AidcError::InvalidPayload(
                "invalid variable measure RCN format".to_owned(),
            ));
        }
        out.push(check_digit(&out)?);
        Ok(out)
    }
}

fn validate_prefix(format: &str, rcn: &str) -> Result<(), AidcError> {
    if rcn.as_bytes()[0] != b'2' {
        return Err(AidcError::InvalidPayload(
            "invalid variable measure RCN prefix".to_owned(),
        ));
    }
    if format.len() == 13 && rcn.as_bytes()[1] != format.as_bytes()[1] {
        return Err(AidcError::InvalidPayload(
            "invalid variable measure RCN prefix".to_owned(),
        ));
    }
    Ok(())
}

fn validate_format_shape(format: &str) -> Result<(), AidcError> {
    if !(format.len() == 12 || format.len() == 13) {
        return Err(AidcError::InvalidPayload(
            "invalid variable measure RCN format".to_owned(),
        ));
    }
    let b = format.as_bytes();
    if b[0] != b'2' {
        return Err(AidcError::InvalidPayload(
            "invalid variable measure RCN format".to_owned(),
        ));
    }
    if format.len() == 13 && !b[1].is_ascii_digit() {
        return Err(AidcError::InvalidPayload(
            "invalid variable measure RCN format".to_owned(),
        ));
    }
    if b[b.len() - 1] != b'C' {
        return Err(AidcError::InvalidPayload(
            "invalid variable measure RCN format".to_owned(),
        ));
    }
    Ok(())
}

fn pad_numeric(value: u32, len: usize) -> Result<String, AidcError> {
    let s = value.to_string();
    if s.len() > len {
        return Err(AidcError::InvalidPayload(
            "invalid variable measure RCN format".to_owned(),
        ));
    }
    Ok(format!("{value:0len$}"))
}

#[cfg(test)]
mod tests {
    use super::{RcnReference, VariableMeasure};
    use crate::check::{has_valid_check_digit, is_valid_price_or_weight_check_digit};

    #[test]
    fn rcn12_vectors() {
        let r1 = VariableMeasure::create_rcn("2IIIIIVPPPPC", 12345, 4321).expect("rcn");
        assert_eq!(r1.len(), 12);
        assert_eq!(&r1[0..1], "2");
        assert_eq!(&r1[1..6], "12345");
        assert_eq!(&r1[7..11], "4321");
        assert!(
            is_valid_price_or_weight_check_digit(&r1[7..11], r1.chars().nth(6).expect("ch"))
                .expect("pw")
        );
        assert!(has_valid_check_digit(&r1).expect("cd"));
        assert_eq!(
            VariableMeasure::parse_rcn("2IIIIIVPPPPC", &r1).expect("parse"),
            RcnReference {
                item_reference: 12345,
                price_or_weight: 4321
            }
        );

        let r2 = VariableMeasure::create_rcn("2IIIIPPPPPVC", 1234, 54321).expect("rcn");
        assert_eq!(
            VariableMeasure::parse_rcn("2IIIIPPPPPVC", &r2).expect("parse"),
            RcnReference {
                item_reference: 1234,
                price_or_weight: 54321
            }
        );

        let r3 = VariableMeasure::create_rcn("2PPPPPIIIIIC", 12345, 54321).expect("rcn");
        assert_eq!(
            VariableMeasure::parse_rcn("2PPPPPIIIIIC", &r3).expect("parse"),
            RcnReference {
                item_reference: 12345,
                price_or_weight: 54321
            }
        );
        assert!(VariableMeasure::create_rcn("3PPPPPIIIIIC", 12345, 54321).is_err());
        assert!(VariableMeasure::parse_rcn("2PPPPPIIIIIC", "254321123454").is_ok());
    }

    #[test]
    fn rcn13_vectors() {
        let r1 = VariableMeasure::create_rcn("24IIIIIVPPPPC", 12345, 4321).expect("rcn");
        assert_eq!(r1.len(), 13);
        assert_eq!(&r1[0..2], "24");
        assert_eq!(
            VariableMeasure::parse_rcn("24IIIIIVPPPPC", &r1).expect("parse"),
            RcnReference {
                item_reference: 12345,
                price_or_weight: 4321
            }
        );
        let r2 = VariableMeasure::create_rcn("21IIIIPPPPPVC", 1234, 54321).expect("rcn");
        assert_eq!(
            VariableMeasure::parse_rcn("21IIIIPPPPPVC", &r2).expect("parse"),
            RcnReference {
                item_reference: 1234,
                price_or_weight: 54321
            }
        );
        assert!(VariableMeasure::create_rcn("30PPPPPIIIIIC", 12345, 54321).is_err());
    }
}
