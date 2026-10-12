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

thread_local! {
    static HERE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Overrides the language for the current thread only (tests run languages side by side).
pub fn set_english_here(on: Option<bool>) {
    HERE.with(|h| h.set(on));
}

pub fn english() -> bool {
    HERE.with(|h| h.get()).unwrap_or_else(|| ENGLISH.load(Ordering::Relaxed))
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
            return fill(en, &caps, &tb.exact);
        }
    }
    s.to_string()
}

/// Puts the captures into an English template: `{}` takes the next one, `{N}` takes number N.
fn fill(en: &str, caps: &[&str], exact: &HashMap<&'static str, &'static str>) -> String {
    let mut out = String::with_capacity(en.len() * 2);
    let mut next = 0;
    let mut rest = en;
    while let Some(open) = rest.find('{') {
        out.push_str(rest.get(..open).unwrap_or(""));
        let after = rest.get(open + 1..).unwrap_or("");
        let Some(close) = after.find('}') else {
            out.push_str(rest.get(open..).unwrap_or(""));
            return out;
        };
        let inside = after.get(..close).unwrap_or("");
        let index = if inside.is_empty() {
            next += 1;
            Some(next - 1)
        } else {
            inside.parse::<usize>().ok()
        };
        match index {
            Some(i) => {
                let c = caps.get(i).copied().unwrap_or("");
                out.push_str(exact.get(c).copied().unwrap_or(c));
            }
            None => out.push_str(rest.get(open..open + close + 2).unwrap_or("")),
        }
        rest = after.get(close + 1..).unwrap_or("");
    }
    out.push_str(rest);
    out
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
        let exact = HashMap::new();
        assert_eq!(fill("{1} of {0}", &["a", "b"], &exact), "b of a");
        set_english(false);
        assert_eq!(ts("Настройки"), "Настройки");
        assert_eq!(t("Цель дня: 30 / 50 XP"), "Цель дня: 30 / 50 XP");
    }

    #[test]
    fn table_templates_have_matching_placeholders() {
        for (ru, en) in en::PAIRS {
            let slots = en.matches("{}").count() + (0..9).filter(|i| en.contains(&format!("{{{i}}}"))).count();
            assert_eq!(ru.matches("{}").count(), slots, "{ru} → {en}");
        }
    }
}
