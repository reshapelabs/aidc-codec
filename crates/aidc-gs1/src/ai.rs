#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AiMeta {
    pub code: &'static str,
    pub fixed_len: Option<u8>,
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
