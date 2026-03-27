use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Meta {
    fixed_len: Option<u8>,
    fnc1_required: bool,
    dl_data_attr: bool,
    dl_primary_key: bool,
    dl_qualifiers: Option<String>,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source = manifest_dir.join("data/gs1-syntax-dictionary.txt");
    println!("cargo:rerun-if-changed={}", source.display());

    let input = fs::read_to_string(&source).expect("read gs1 dictionary");
    let mut table = BTreeMap::<String, Meta>::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut split = line.splitn(2, '#');
        let body = split.next().unwrap_or_default().trim();
        if body.is_empty() {
            continue;
        }

        let tokens: Vec<&str> = body.split_whitespace().collect();
        if tokens.len() < 3 {
            continue;
        }

        let ai_token = tokens[0];
        let flags = tokens[1];

        let mut spec_tokens = Vec::new();
        let mut attr_tokens = Vec::new();
        for t in &tokens[2..] {
            let is_attr = t == &"dlpkey" || t.contains('=');
            if is_attr || !attr_tokens.is_empty() {
                attr_tokens.push(*t);
            } else {
                spec_tokens.push(*t);
            }
        }
        if spec_tokens.is_empty() {
            continue;
        }

        let spec = spec_tokens.join(" ");
        let attrs = attr_tokens.join(" ");
        let fixed_len = fixed_len_from_spec(&spec);
        let dl_primary_key = attrs.contains("dlpkey");
        let dl_qualifiers = attrs
            .split_whitespace()
            .find_map(|a| a.strip_prefix("dlpkey=").map(str::to_owned));

        let meta = Meta {
            fixed_len,
            fnc1_required: !flags.contains('*'),
            dl_data_attr: flags.contains('?'),
            dl_primary_key,
            dl_qualifiers,
        };

        for ai in expand_ai(ai_token) {
            table.entry(ai).or_insert_with(|| meta.clone());
        }
    }

    let mut out = String::new();
    out.push_str("pub static AI_DICTIONARY: phf::Map<&'static str, AiMeta> = phf::phf_map! {\n");
    for (ai, m) in &table {
        let fixed = match m.fixed_len {
            Some(n) => format!("Some({n})"),
            None => "None".to_owned(),
        };
        let quals = match &m.dl_qualifiers {
            Some(q) => format!("Some({q:?})"),
            None => "None".to_owned(),
        };
        out.push_str(&format!(
            "    {ai:?} => AiMeta {{ code: {ai:?}, fixed_len: {fixed}, fnc1_required: {}, dl_data_attr: {}, dl_primary_key: {}, dl_qualifiers: {} }},\n",
            m.fnc1_required, m.dl_data_attr, m.dl_primary_key, quals
        ));
    }
    out.push_str("};\n");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("ai_dictionary.rs");
    fs::write(out_path, out).expect("write generated ai dictionary");
}

fn expand_ai(token: &str) -> Vec<String> {
    if let Some((start, end)) = token.split_once('-') {
        if let (Ok(a), Ok(b)) = (start.parse::<u32>(), end.parse::<u32>()) {
            let width = start.len();
            return (a..=b).map(|n| format!("{n:0width$}")).collect();
        }
    }
    vec![token.to_owned()]
}

fn fixed_len_from_spec(spec: &str) -> Option<u8> {
    let mut sum: u16 = 0;
    for part in spec.split_whitespace() {
        let mut p = part;
        if p.starts_with('[') {
            return None;
        }
        if let Some((head, _)) = p.split_once(',') {
            p = head;
        }
        if p.len() < 2 {
            return None;
        }
        let len = &p[1..];
        if len.contains("..") {
            return None;
        }
        let n = len.parse::<u16>().ok()?;
        sum = sum.checked_add(n)?;
    }
    u8::try_from(sum).ok()
}
