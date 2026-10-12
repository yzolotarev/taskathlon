//! Interface language. Source strings are Russian; with English switched on, [`ts`] and [`t`]
//! look them up in the English table. A formatted string is matched against templates where
//! `{}` stands for any value, so `t(&format!("Уровень {}: {}", n, name))` translates too.
//! Anything missing from the table is shown as it is.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable)]

mod en;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set_english(on: bool) {
    ENGLISH.store(on, Ordering::Relaxed);
    if let Ok(mut c) = cache().lock() {
        c.clear();
    }
}

pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

struct Tables {
    exact: HashMap<&'static str, &'static str>,
    /// (literal parts of the Russian template, English template), most specific first.
    templates: Vec<(Vec<&'static str>, &'static str)>,
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut exact = HashMap::new();
        let mut templates = Vec::new();
        for (ru, en) in en::PAIRS {
            if ru.contains("{}") {
                let parts: Vec<&'static str> = ru.split("{}").collect();
                if parts.iter().map(|p| p.len()).sum::<usize>() > 0 {
                    templates.push((parts, *en));
                }
            } else {
                exact.insert(*ru, *en);
            }
        }
        templates.sort_by_key(|(parts, _)| std::cmp::Reverse(parts.iter().map(|p| p.chars().count()).sum::<usize>()));
        Tables { exact, templates }
    })
}

fn cache() -> &'static Mutex<HashMap<String, String>> {
    static C: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Translates a fixed string.
pub fn ts(ru: &'static str) -> &'static str {
    if !english() {
        return ru;
    }
    tables().exact.get(ru).copied().unwrap_or(ru)
}

/// The values a template's `{}` stand for, if `s` fits the template.
fn captures<'a>(parts: &[&str], s: &'a str) -> Option<Vec<&'a str>> {
    let (first, rest) = parts.split_first()?;
    let (last, middle) = rest.split_last()?;
    let mut body = s.strip_prefix(first)?;
    body = body.strip_suffix(last)?;
    let mut caps = Vec::with_capacity(rest.len());
    for m in middle {
        if m.is_empty() {
            return None;
        }
        let at = body.find(m)?;
        caps.push(body.get(..at)?);
        body = body.get(at + m.len()..)?;
    }
    caps.push(body);
    Some(caps)
}

fn translate(s: &str) -> String {
    let tb = tables();
    if let Some(en) = tb.exact.get(s) {
        return (*en).to_string();
    }
    for (parts, en) in &tb.templates {
        if let Some(caps) = captures(parts, s) {
            let mut out = String::with_capacity(en.len() + s.len());
            let mut caps = caps.into_iter();
            for (i, piece) in en.split("{}").enumerate() {
                if i > 0 {
                    let c = caps.next().unwrap_or("");
                    out.push_str(tb.exact.get(c).copied().unwrap_or(c));
                }
                out.push_str(piece);
            }
            return out;
        }
    }
    s.to_string()
}

/// Translates a formatted string.
pub fn t(s: &str) -> String {
    if !english() {
        return s.to_string();
    }
    if let Ok(mut c) = cache().lock() {
        if let Some(hit) = c.get(s) {
            return hit.clone();
        }
        let out = translate(s);
        if c.len() > 4096 {
            c.clear();
        }
        c.insert(s.to_string(), out.clone());
        return out;
    }
    translate(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_template_lookups() {
        set_english(true);
        assert_eq!(ts("Настройки"), "Settings");
        assert_eq!(t("Цель дня: 30 / 50 XP"), "Daily goal: 30 / 50 XP");
        assert_eq!(t("Уровень 2: С подсказкой"), "Level 2: With a hint");
        assert_eq!(t("нет такого"), "нет такого");
        set_english(false);
        assert_eq!(ts("Настройки"), "Настройки");
        assert_eq!(t("Цель дня: 30 / 50 XP"), "Цель дня: 30 / 50 XP");
    }

    #[test]
    fn table_templates_have_matching_placeholders() {
        for (ru, en) in en::PAIRS {
            assert_eq!(ru.matches("{}").count(), en.matches("{}").count(), "{ru} → {en}");
        }
    }
}
