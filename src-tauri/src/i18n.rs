//! Language of the strings the backend produces (diagnosis, notifications, tray menu, errors):
//! the "core" section of the same Italian and English JSON dictionaries as the interface.
//! `t("section.key")`, `tf("key", &[("name", value)])`, `tn("key", count, &[...])` for plurals.
use serde_json::Value;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

const IT: u8 = 0;
const EN: u8 = 1;
static LANG: AtomicU8 = AtomicU8::new(EN);

fn core(json: &str) -> Value {
    serde_json::from_str::<Value>(json).ok().and_then(|mut v| v.get_mut("core").map(Value::take)).unwrap_or(Value::Null)
}

fn dicts() -> &'static [Value; 2] {
    static D: OnceLock<[Value; 2]> = OnceLock::new();
    D.get_or_init(|| {
        [
            core(include_str!("../../src/locales/it.json")),
            core(include_str!("../../src/locales/en.json")),
        ]
    })
}

/// System language from the locale environment variables: Italian if it starts with "it", English otherwise.
pub fn system_lang() -> &'static str {
    let v = ["LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty())).unwrap_or_default();
    if v.to_lowercase().starts_with("it") {
        "it"
    } else {
        "en"
    }
}

/// Applies the preference from the settings: "it", "en" or "auto" (system language).
pub fn set(pref: &str) {
    let l = match pref {
        "it" => IT,
        "en" => EN,
        _ if system_lang() == "it" => IT,
        _ => EN,
    };
    LANG.store(l, Ordering::Relaxed);
}

pub fn lang() -> &'static str {
    if LANG.load(Ordering::Relaxed) == IT {
        "it"
    } else {
        "en"
    }
}

fn lookup<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    key.split('.').try_fold(v, |v, k| v.get(k))
}

fn raw(key: &str, count: Option<i64>) -> String {
    let d = dicts();
    let l = LANG.load(Ordering::Relaxed) as usize;
    let v = lookup(&d[l], key).or_else(|| lookup(&d[EN as usize], key)).or_else(|| lookup(&d[IT as usize], key));
    match (v, count) {
        (Some(Value::String(s)), _) => s.clone(),
        (Some(Value::Object(o)), Some(n)) => o.get(if n == 1 { "one" } else { "other" }).or_else(|| o.get("other")).and_then(|v| v.as_str()).unwrap_or(key).to_string(),
        _ => key.to_string(),
    }
}

/// Fills the {placeholders} in a single pass, so a value that contains braces (an app name) is never expanded again.
fn fill(s: String, args: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut rest = s.as_str();
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        match tail.find('}').and_then(|j| args.iter().find(|(k, _)| *k == &tail[..j]).map(|(_, v)| (j, v))) {
            Some((j, v)) => {
                out.push_str(v);
                rest = &tail[j + 1..];
            }
            None => {
                out.push('{');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn t(key: &str) -> String {
    raw(key, None)
}

/// The value of a key in every language (to recognize a name whatever language produced it).
pub fn all(key: &str) -> Vec<String> {
    dicts().iter().filter_map(|d| lookup(d, key).and_then(|v| v.as_str()).map(String::from)).collect()
}

pub fn tf(key: &str, args: &[(&str, String)]) -> String {
    fill(raw(key, None), args)
}

/// Plural: the value is {"one": "...", "other": "..."}; {count} is filled in too.
pub fn tn(key: &str, count: i64, args: &[(&str, String)]) -> String {
    let mut all: Vec<(&str, String)> = vec![("count", count.to_string())];
    all.extend(args.iter().cloned());
    fill(raw(key, Some(count)), &all)
}
