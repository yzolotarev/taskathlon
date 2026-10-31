//! Interface language. Source strings are Russian; with English switched on, [`ts`] and [`t`]
//! look them up in the English table. A formatted string is matched against templates where
//! `{}` stands for any value, so `t(&format!("Уровень {}: {}", n, name))` translates too.
//! Anything missing from the table is shown as it is.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable)]

mod en;

// Procurement case additions live here alongside the translation mechanism.
const PROCUREMENT_PAIRS: &[(&str, &str)] = &[
    ("Цена или сообщение", "Price or message"),
    ("Без ошибки", "Without error"),
    ("Результат ВПР", "VLOOKUP result"),
    ("Увидеть #Н/Д", "See #N/A"),
    ("нет", "none"),
    ("Когда код не найден", "When a code is missing"),
    (
        "Точное совпадение ВПР даёт #Н/Д, если такого кода нет. Сначала замечай ошибку, не маскируй её.",
        "An exact-match VLOOKUP returns #N/A when a code is absent. First notice the error; do not hide it.",
    ),
    ("ВПР(код; каталог; столбец; ЛОЖЬ) → #Н/Д", "VLOOKUP(code, catalog, column, FALSE) → #N/A"),
    ("Обернуть результат в ЕСЛИОШИБКА", "Wrap a result in IFERROR"),
    (
        "ЕСЛИОШИБКА получает уже вычисленный результат: если это ошибка — подставляет понятную замену.",
        "IFERROR takes an already-computed result: if it is an error, return a clear replacement.",
    ),
    ("=ЕСЛИОШИБКА(готовый_результат; \"нет\")", "=IFERROR(existing_result, \"none\")"),
    ("Соединить ВПР и ЕСЛИОШИБКА", "Combine VLOOKUP and IFERROR"),
    (
        "ВПР ищет цену, внешняя ЕСЛИОШИБКА обрабатывает только случай отсутствующего артикула.",
        "VLOOKUP finds the price; outer IFERROR handles only a missing SKU.",
    ),
    ("=ЕСЛИОШИБКА(ВПР(...); \"нет\")", "=IFERROR(VLOOKUP(...), \"none\")"),
    (
        "Код из {} есть в каталоге, кода из {} нет. Введи ВПР в обе жёлтые ячейки: каждая формула ищет код из своей строки. Посмотри, что вернёт Excel.",
        "The code in {} is in the catalog, the code in {} is not. Enter VLOOKUP in both yellow cells: each formula looks up the code from its own row. See what Excel returns.",
    ),
    (
        "В {} и {} найди цену через ВПР с точным совпадением, каждая ячейка ищет код из своей строки. Один код есть в каталоге, другого нет: посмотри, что вернёт Excel.",
        "In {} and {}, find the price with an exact-match VLOOKUP; each cell looks up the code from its own row. One code is in the catalog, the other is not: see what Excel returns.",
    ),
    (
        "Код {} есть в каталоге (строка {}), поэтому в {} получится его цена. Кода {} в каталоге нет, поэтому в {} будет #Н/Д. Так и задумано.",
        "Code {} is in the catalog (row {}), so {} gets its price. Code {} is not in the catalog, so {} shows #N/A. That is the point.",
    ),
    (
        "Формулы в {} и {} отличаются только первой частью: в {} ищем код из {}, в {} ищем код из {}.",
        "The formulas in {} and {} differ only in the first part: {} looks up the code from {}, {} looks up the code from {}.",
    ),
    (
        "Что значит каждая часть: {} - что ищем, {} - каталог, 3 - номер столбца с ценой, {} - только точное совпадение",
        "What each part means: {} - what to find, {} - the catalog, 3 - the price column number, {} - exact match only",
    ),
    (
        "Формула в {} берёт {}, а здесь нужна {}: ссылка указывает не на ту ячейку.",
        "The formula in {} uses {}, but this cell needs {}: the reference points to the wrong cell.",
    ),
    (
        "В столбце {} результат ВПР уже вычислен. В {} и {} оберни готовый результат в ЕСЛИОШИБКА(...;\"нет\"). Сравни строку с найденным кодом и строку с #Н/Д.",
        "The VLOOKUP result is already calculated in column {}. In {} and {}, wrap that result in IFERROR(...,\"none\"). Compare the found-code row with the #N/A row.",
    ),
    (
        "Введи формулу в {} и протяни до {}. Сначала ВПР ищет точный код, внешняя ЕСЛИОШИБКА возвращает «нет», если кода нет.",
        "Enter the formula in {} and fill down to {}. VLOOKUP first searches for an exact code; outer IFERROR returns \"none\" when it is missing.",
    ),
    (
        "Подтяни цены в {}. Соедини знакомую ВПР с ЕСЛИОШИБКА; проверь существующий и отсутствующий код.",
        "Look up prices in {}. Combine the familiar VLOOKUP with IFERROR; check both existing and missing codes.",
    ),
    (
        "Подготовь ответ по каждому артикулу в {}. Отсутствующий код отметь словом «нет», а не нулём.",
        "Prepare an answer for each SKU in {}. Mark a missing code with the word \"none\", not with zero.",
    ),
    ("Артикул", "SKU"),
    ("Заказ", "Order"),
    ("Заказан", "Ordered"),
    ("Получен", "Received"),
    ("Общая сумма", "Grand total"),
    ("Сумма закупок", "Purchase total"),
    ("Реестр закупок: от каталога до отчёта", "Purchase register: from catalog to report"),
    ("Специалист по снабжению", "Procurement specialist"),
    ("{} — строка {}", "{} — row {}"),
    ("Закупки поставщика (O{})", "Supplier purchases (O{})"),
    ("Общая сумма (O7)", "Grand total (O7)"),
    ("Независимый контроль бюджета", "Independent budget check"),
    ("Контрольный срез: поставщик / статус", "Control breakdown: supplier / status"),
    ("Настоящая сводная: поставщик × статус", "Actual pivot table: supplier × status"),
    ("Руководитель отдела снабжения", "Head of procurement"),
    ("Закупки к еженедельной планёрке", "Purchases for the weekly meeting"),
    (
        "В A:G — заказы, в I:L — каталог. Подтяни цену и поставщика по точному артикулу, рассчитай суммы и подготовь отчёт по поставщикам и статусам. Книга должна пересчитываться при изменении цены или количества. Исходные данные не меняй.",
        "A:G contains orders; I:L contains the catalog. Look up the price and supplier by exact SKU, calculate totals, and prepare a report by supplier and status. The workbook must recalculate when prices or quantities change. Do not change the source data.",
    ),
    (
        "D2:D{} и E2:E{} — цена и поставщик из каталога, живыми формулами точного поиска.",
        "D2:D{} and E2:E{} — price and supplier from the catalog, using live exact-match lookup formulas.",
    ),
    ("G2:G{} — цена × количество; напиши первую формулу и протяни вниз.", "G2:G{} — price × quantity; write the first formula and fill down."),
    (
        "O2:O4 — суммы по поставщикам через СУММЕСЛИ; O7 — общий итог. Проверь, что сумма O2:O4 равна O7.",
        "O2:O4 — supplier totals using SUMIF; O7 — grand total. Check that the sum of O2:O4 equals O7.",
    ),
    (
        "В свободной области Q1 создай настоящую сводную: строки — поставщик, столбцы — статус; значения — сумма закупок (СУММА, не СЧЁТ). Источник — весь реестр A:G вместе с заголовками.",
        "Create an actual pivot table in the free area at Q1: rows — supplier, columns — status; values — purchase total (SUM, not COUNT). Use the entire A:G register, including headers, as the source.",
    ),
    (
        "Для независимого числового контроля рядом выведи шесть строк «поставщик / статус» и сумму справа. Можно использовать формулы. Проверяются конфигурация настоящей сводной и контрольные суммы; обновление сводной проверяется отдельно.",
        "For an independent numerical check, add six supplier / status rows nearby with totals on the right. You may use formulas. The actual pivot table configuration and control totals are checked; pivot refresh is checked separately.",
    ),
];

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
        for (ru, en) in en::PAIRS.iter().chain(PROCUREMENT_PAIRS) {
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
        for (ru, en) in en::PAIRS.iter().chain(PROCUREMENT_PAIRS) {
            let slots = en.matches("{}").count() + (0..9).filter(|i| en.contains(&format!("{{{i}}}"))).count();
            assert_eq!(ru.matches("{}").count(), slots, "{ru} → {en}");
        }
    }

    #[test]
    fn procurement_translations_are_unique_and_complete() {
        let mut keys = std::collections::HashSet::new();
        for (ru, _) in en::PAIRS {
            keys.insert(*ru);
        }
        for (ru, en) in PROCUREMENT_PAIRS {
            assert!(keys.insert(*ru), "duplicate translation: {ru}");
            assert_eq!(translate(ru), *en);
            assert!(!en.chars().any(|c| ('\u{0400}'..='\u{04ff}').contains(&c)), "{en}");
        }
        assert_eq!(
            translate("D2:D13 и E2:E13 — цена и поставщик из каталога, живыми формулами точного поиска."),
            "D2:D13 and E2:E13 — price and supplier from the catalog, using live exact-match lookup formulas."
        );
        assert_eq!(
            translate("G2:G13 — цена × количество; напиши первую формулу и протяни вниз."),
            "G2:G13 — price × quantity; write the first formula and fill down."
        );
        assert_eq!(translate("Закупки поставщика (O2)"), "Supplier purchases (O2)");
        assert_eq!(translate("D2 — строка 2"), "D2 — row 2");
    }
}
