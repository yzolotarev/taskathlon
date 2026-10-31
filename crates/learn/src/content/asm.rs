//! Report assembly: one tidy report sheet built from messy source sheets.
//!
//! Work rarely starts from a clean table. A 1С export, a colleague's hand-kept sheet and a
//! supplier's price list differ in column order, header names, date and number formats and
//! spellings. Each skill here drills one movement of that job on fresh variations, and
//! `asm.report` puts the movements together. The report is graded row by row against an
//! independent truth, whatever the method: copying, formulas or retyping.

use super::Gen;
use super::live::{ITEMS, SIDE_SUPPLIER, SUPPLIERS};
use crate::check::{Col, Expectation, Kind};
use crate::sheet::{Cell, a1, date, date_text};
use crate::{Result, Task};

pub const REPORT: &str = "Отчёт";
const MONTH_NAMES: [&str; 12] = ["Январь", "Февраль", "Март", "Апрель", "Май", "Июнь", "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь", "Декабрь"];
const MONTHS: [&str; 12] = ["января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября", "октября", "ноября", "декабря"];

/// One purchase order, the truth every source and the report are derived from.
#[derive(Clone, Debug)]
struct Rec {
    no: String,
    date: f64,
    due: f64,
    sup: &'static str,
    item: usize,
    qty: f64,
    price: f64,
}

impl Rec {
    fn art(&self) -> String {
        format!("OM-{}", 101 + self.item)
    }
    fn name(&self) -> &'static str {
        ITEMS.get(self.item).map(|x| x.0).unwrap_or("Товар")
    }
    fn amount(&self) -> f64 {
        self.qty * self.price
    }
}

/// A report or source column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum F {
    No,
    Date,
    Due,
    Sup,
    Item,
    Art,
    Qty,
    Price,
    Amount,
    /// The month a row came from: the sheet it was copied off, filled in by the learner.
    Month,
    Days,
    /// Source-only noise columns.
    Store,
    Person,
}

impl F {
    fn header(self) -> &'static str {
        match self {
            F::No => "№ заказа",
            F::Date => "Дата",
            F::Due => "Срок поставки",
            F::Sup => "Поставщик",
            F::Item => "Товар",
            F::Art => "Артикул",
            F::Qty => "Кол-во",
            F::Price => "Цена",
            F::Amount => "Сумма",
            F::Month => "Месяц",
            F::Days => "Дней в пути",
            F::Store => "Склад",
            F::Person => "Ответственный",
        }
    }
    /// What other people call the same column.
    fn aliases(self) -> &'static [&'static str] {
        match self {
            F::No => &["Номер", "№ док.", "Заказ №"],
            F::Date => &["Дата док.", "Дата заказа", "Дата"],
            F::Due => &["Ожидаем", "Срок", "Поставка до"],
            F::Sup => &["Контрагент", "Поставщик", "Продавец"],
            F::Item => &["Наименование", "Номенклатура", "Товар"],
            F::Art => &["Код", "Арт.", "Артикул"],
            F::Qty => &["Кол.", "Количество", "Шт."],
            F::Price => &["Цена за ед.", "Цена, тг", "Цена"],
            F::Amount => &["Сумма, тг", "Итого по строке", "Сумма"],
            F::Month => &["Месяц"],
            F::Days => &["Дней"],
            F::Store => &["Склад"],
            F::Person => &["Ответственный", "Менеджер"],
        }
    }
    fn col(self) -> Col {
        match self {
            F::Date | F::Due => Col::Date,
            F::Qty | F::Price | F::Amount | F::Days => Col::Number,
            _ => Col::Text,
        }
    }
    fn truth(self, r: &Rec) -> Cell {
        match self {
            F::No => Cell::Text(r.no.clone()),
            F::Date => Cell::Date(r.date),
            F::Due => Cell::Date(r.due),
            F::Sup => Cell::Text(r.sup.into()),
            F::Item => Cell::Text(r.name().into()),
            F::Art => Cell::Text(r.art()),
            F::Qty => Cell::Num(r.qty),
            F::Price => Cell::Num(r.price),
            F::Amount => Cell::Num(r.amount()),
            F::Month => {
                let m = crate::sheet::ymd(r.date).map_or(3, |(_, m, _)| m);
                Cell::Text(MONTH_NAMES.get((m - 1).clamp(0, 11) as usize).copied().unwrap_or("Март").into())
            }
            F::Days => Cell::Num(r.due - r.date),
            F::Store => Cell::Text("Алматы".into()),
            F::Person => Cell::Text("Айгерим".into()),
        }
    }
    fn expect(self, r: &Rec) -> Expectation {
        match self.truth(r) {
            Cell::Num(n) | Cell::Date(n) => Expectation::Number(n),
            Cell::Text(t) => Expectation::Text(t),
            Cell::Formula(_) => Expectation::Empty,
        }
    }
}

/// How a value was typed into a source. Everything but `Clean` and `IsoDate` needs work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mess {
    Clean,
    /// A real date shown as yyyy-mm-dd: looks different, is fine.
    IsoDate,
    TextDate,
    ShortTextDate,
    WordDate,
    NumText,
    NumUnit,
    NameVariant,
    NameCase,
    NameSpaces,
}

/// Renders a value the way `mess` says; returns the cell, an optional number format and
/// why it needs work (None when it does not).
fn render(f: F, r: &Rec, mess: Mess, pick: usize) -> (Cell, Option<&'static str>, Option<String>) {
    let serial = match f {
        F::Due => r.due,
        _ => r.date,
    };
    let (y, m, d) = crate::sheet::ymd(serial).unwrap_or((2026, 3, 1));
    match (mess, f.col()) {
        (Mess::IsoDate, Col::Date) => (Cell::Date(serial), Some("yyyy-mm-dd"), None),
        (Mess::TextDate, Col::Date) => {
            let t = date_text(serial);
            (Cell::Text(format!("'{t}")), None, Some(format!("«{t}» записана текстом (прижата к левому краю)")))
        }
        (Mess::ShortTextDate, Col::Date) => {
            let t = format!("{d:02}.{m:02}.{}", y % 100);
            (Cell::Text(format!("'{t}")), None, Some(format!("«{t}» — текст с коротким годом")))
        }
        (Mess::WordDate, Col::Date) => {
            let t = format!("{d} {}", MONTHS.get((m - 1).clamp(0, 11) as usize).copied().unwrap_or("марта"));
            (Cell::Text(t.clone()), None, Some(format!("«{t}» записана словами — Excel не считает это датой")))
        }
        (Mess::NumText, Col::Number) => {
            let n = match f.truth(r) {
                Cell::Num(n) => n,
                _ => 0.0,
            };
            (Cell::Text(format!("'{}", n as i64)), None, Some(format!("«{}» — число, записанное текстом", n as i64)))
        }
        (Mess::NumUnit, Col::Number) => {
            let n = match f.truth(r) {
                Cell::Num(n) => n,
                _ => 0.0,
            };
            let unit = if f == F::Qty { "шт" } else { "тг" };
            let t = format!("{} {unit}", n as i64);
            (Cell::Text(t.clone()), None, Some(format!("«{t}» — с «{unit}» это текст, а не число")))
        }
        (Mess::NameVariant, Col::Text) if f == F::Sup => {
            let core = r.sup.strip_prefix("ТОО ").unwrap_or(r.sup);
            let t = match pick % 3 {
                0 => format!("{core} ТОО"),
                1 => format!("ТОО «{core}»"),
                _ => core.to_string(),
            };
            (Cell::Text(t.clone()), None, Some(format!("«{t}» — по справочнику «{}»", r.sup)))
        }
        (Mess::NameCase, Col::Text) if f == F::Sup => {
            let t = r.sup.to_lowercase();
            (Cell::Text(t.clone()), None, Some(format!("«{t}» — в отчёте пишем как в справочнике: «{}»", r.sup)))
        }
        (Mess::NameSpaces, Col::Text) if f == F::Sup => {
            let t = format!(" {}", r.sup.replacen(' ', "  ", 1));
            (Cell::Text(t), None, None)
        }
        _ => {
            let fmt = if f.col() == Col::Date { Some("dd.mm.yyyy") } else { None };
            (f.truth(r), fmt, None)
        }
    }
}

fn q(sheet: &str, row: u32, col: u32) -> String {
    format!("'{sheet}'!{}", a1(row, col))
}

/// Writes a source sheet: optional title, shuffled columns under other people's headers,
/// one row per record, with the dirt `mess` asks for. Notes go to (key, report column).
#[allow(clippy::too_many_arguments)]
fn source(
    g: &mut Gen,
    sheet: &str,
    title: Option<&str>,
    fields: &[F],
    recs: &[&Rec],
    report: &[F],
    mess: &mut dyn FnMut(F, usize) -> Mess,
    notes: &mut Vec<(String, usize, String)>,
) {
    let mut order: Vec<F> = fields.to_vec();
    g.l.shuffle(&mut order);
    let header = if title.is_some() { 3 } else { 1 };
    if let Some(t) = title {
        g.setup.text(q(sheet, 1, 0), t);
        g.setup.bold.push(q(sheet, 1, 0));
    }
    for (c, f) in order.iter().enumerate() {
        let names = f.aliases();
        let name = names.get(g.l.below(names.len())).copied().unwrap_or(f.header());
        g.setup.text(q(sheet, header, c as u32), name);
        let w = if matches!(f, F::Sup | F::Item) { 180.0 } else { 105.0 };
        g.setup.widths.push((format!("'{sheet}'!{0}:{0}", crate::sheet::col_name(c as u32)), w));
    }
    g.setup.bold.push(format!("'{sheet}'!{}:{}", a1(header, 0), a1(header, order.len() as u32 - 1)));
    for (i, r) in recs.iter().enumerate() {
        let row = header + 1 + i as u32;
        for (c, f) in order.iter().enumerate() {
            let m = mess(*f, i);
            let (cell, fmt, why) = render(*f, r, m, i + c);
            let at = q(sheet, row, c as u32);
            if let Some(fmt) = fmt {
                g.setup.formats.push((at.clone(), fmt.into()));
            }
            // Dates go in as serials; the format above decides how they look.
            let cell = match cell {
                Cell::Date(n) => Cell::Num(n),
                other => other,
            };
            g.setup.input(at, cell);
            if let (Some(why), Some(rc)) = (why, report.iter().position(|x| x == f)) {
                notes.push((r.no.clone(), rc, format!("на листе «{sheet}», строка {row}: {why}")));
            }
        }
    }
}

fn records(g: &mut Gen, n: usize) -> Vec<Rec> {
    let mut sups: Vec<&'static str> = SUPPLIERS.to_vec();
    g.l.shuffle(&mut sups);
    sups.truncate(3);
    sups.push(SIDE_SUPPLIER);
    let start = 200 + g.d.int(0, 600);
    let mut day = date(2026, 3, 2) + g.d.int(0, 4) as f64;
    (0..n)
        .map(|i| {
            let item = g.d.below(ITEMS.len());
            let base = ITEMS.get(item).map(|x| x.1).unwrap_or(1_000);
            let qty = if base < 1_000 {
                g.d.int(20, 200)
            } else if base > 20_000 {
                g.d.int(1, 6)
            } else {
                g.d.int(2, 30)
            } as f64;
            let rec = Rec {
                no: format!("PO-{}", start + i as i64),
                date: day,
                due: day + g.d.int(3, 21) as f64,
                sup: sups.get(g.d.below(sups.len())).copied().unwrap_or(SIDE_SUPPLIER),
                item,
                qty,
                price: base as f64,
            };
            day += g.d.int(0, 2) as f64;
            rec
        })
        .collect()
}

fn size(level: u8) -> usize {
    match level {
        1 => 4,
        2 => 6,
        3 => 8,
        _ => 11,
    }
}

/// The empty report template on the first sheet and its row-by-row check.
fn template(g: &mut Gen, fields: &[F], recs: &[Rec], notes: Vec<(String, usize, String)>, given: &[F]) {
    g.setup.first_sheet = Some(REPORT.into());
    let headers: Vec<&str> = fields.iter().map(|f| f.header()).collect();
    g.setup.header(1, 0, &headers);
    for (c, f) in fields.iter().enumerate() {
        let w = if matches!(f, F::Sup | F::Item) { 190.0 } else { 110.0 };
        g.setup.width(c as u32, w);
        let range = format!("{}:{}", a1(2, c as u32), a1(recs.len() as u32 + 1, c as u32));
        match f.col() {
            Col::Date => g.setup.formats.push((range, "dd.mm.yyyy".into())),
            Col::Number if matches!(f, F::Price | F::Amount) => g.setup.formats.push((range, "#,##0".into())),
            _ => {}
        }
    }
    for (i, r) in recs.iter().enumerate() {
        for (c, f) in fields.iter().enumerate() {
            if given.contains(f) {
                let cell = match f.truth(r) {
                    Cell::Date(n) => Cell::Num(n),
                    other => other,
                };
                g.setup.input(a1(2 + i as u32, c as u32), cell);
            }
        }
    }
    let cols = fields.iter().map(|f| if given.contains(f) { Col::Given } else { f.col() }).collect();
    let rows = recs.iter().map(|r| fields.iter().map(|f| f.expect(r)).collect()).collect();
    g.check("Отчёт", Kind::Table { first: 2, left: 0, headers: headers.iter().map(|h| h.to_string()).collect(), cols, rows, key: 0, notes });
}

fn finish(g: &mut Gen, title: &str, prompt: String, example: &str) -> Task {
    let level = g.level;
    let mut task = g.task(title, prompt, Some(example.to_string()), "A2".into());
    task.reorders = true;
    task.goal_seconds = 60.0 + 45.0 * f32::from(level);
    task
}

/// Which records go where: the template's own order is never the source's order.
fn shuffled(g: &mut Gen, recs: &[Rec]) -> Vec<Rec> {
    let mut v = recs.to_vec();
    g.l.shuffle(&mut v);
    v
}

pub fn columns(g: &mut Gen) -> Result<Task> {
    let n = size(g.level);
    let recs = records(g, n);
    let report: Vec<F> = match g.level {
        1 => vec![F::No, F::Sup, F::Qty],
        2 => vec![F::No, F::Date, F::Sup, F::Item, F::Qty],
        _ => vec![F::No, F::Date, F::Sup, F::Item, F::Qty, F::Price],
    };
    let mut fields = report.clone();
    if g.level >= 2 {
        fields.extend([F::Store, F::Person]);
    }
    if g.level >= 3 {
        fields.push(F::Art);
    }
    let src = shuffled(g, &recs);
    let refs: Vec<&Rec> = src.iter().collect();
    let iso = g.level >= 3;
    let mut notes = Vec::new();
    let title = (g.level >= 4).then_some("Выгрузка из 1С: заказы поставщикам, март 2026");
    source(g, "Выгрузка 1С", title, &fields, &refs, &report, &mut |f, _| if iso && f == F::Date { Mess::IsoDate } else { Mess::Clean }, &mut notes);
    template(g, &report, &recs, notes, &[]);
    let prompt = match g.level {
        1 => "Перенеси данные с листа «Выгрузка 1С» в шаблон на листе «Отчёт»: каждый столбец — под свой заголовок. В выгрузке столбцы называются иначе и стоят в другом порядке.".to_string(),
        2 => "Заполни «Отчёт» данными с листа «Выгрузка 1С». Лишние столбцы выгрузки в отчёт не нужны.".to_string(),
        _ => "Андрей: «Сведи выгрузку из 1С в наш шаблон отчёта — только нужные столбцы, в нашем порядке».".to_string(),
    };
    Ok(finish(
        g,
        "Перенос по столбцам",
        prompt,
        "«Контрагент» в выгрузке = «Поставщик» в отчёте: выдели данные столбца, Ctrl+C, встань под нужный заголовок отчёта, Ctrl+V.",
    ))
}

pub fn numbers(g: &mut Gen) -> Result<Task> {
    let n = size(g.level).max(5);
    let recs = records(g, n);
    let mut report = vec![F::No, F::Item, F::Qty, F::Price];
    if g.level >= 3 {
        report.push(F::Amount);
    }
    let src = shuffled(g, &recs);
    let refs: Vec<&Rec> = src.iter().collect();
    let dirty = match g.level {
        1 => 2,
        2 => 3,
        3 => 4,
        _ => 6,
    };
    let kinds: Vec<Mess> = if g.level == 1 { vec![Mess::NumUnit] } else { vec![Mess::NumUnit, Mess::NumText] };
    let mut spots: Vec<(usize, F, Mess)> = Vec::new();
    for k in 0..dirty {
        let row = (k * 3 + g.l.below(2)) % n;
        let f = if k % 2 == 0 { F::Qty } else { F::Price };
        let m = kinds.get(k % kinds.len()).copied().unwrap_or(Mess::NumUnit);
        if !spots.iter().any(|(r, ff, _)| *r == row && *ff == f) {
            spots.push((row, f, m));
        }
    }
    let mut notes = Vec::new();
    source(
        g,
        "Заявка склада",
        None,
        &[F::No, F::Item, F::Qty, F::Price],
        &refs,
        &report,
        &mut |f, i| spots.iter().find(|(r, ff, _)| *r == i && *ff == f).map(|s| s.2).unwrap_or(Mess::Clean),
        &mut notes,
    );
    template(g, &report, &recs, notes, &[]);
    let amount = if g.level >= 3 { " Сумму посчитай: кол-во × цена." } else { "" };
    let prompt = format!(
        "Заявку набирали вручную: в части ячеек числа записаны с «шт» и «тг» или как текст. Перенеси её в «Отчёт» так, чтобы кол-во и цена были настоящими числами.{amount}"
    );
    Ok(finish(
        g,
        "Числа из текста",
        prompt,
        "«5 шт» → 5: исправь ячейку руками или =ЗНАЧЕН(ПОДСТАВИТЬ(C4;\" шт\";\"\")). Число-текст (прижато влево) → =ЗНАЧЕН(C4) или перепечатай.",
    ))
}

pub fn dates(g: &mut Gen) -> Result<Task> {
    let n = size(g.level).max(5);
    let recs = records(g, n);
    let mut report = vec![F::No, F::Date, F::Due];
    if g.level >= 4 {
        report.push(F::Days);
    }
    let src = shuffled(g, &recs);
    let refs: Vec<&Rec> = src.iter().collect();
    let kinds: Vec<Mess> = match g.level {
        1 => vec![Mess::TextDate],
        2 => vec![Mess::TextDate, Mess::WordDate],
        _ => vec![Mess::TextDate, Mess::WordDate, Mess::ShortTextDate, Mess::IsoDate],
    };
    let every = if g.level >= 3 { 2 } else { 3 };
    let offset = g.l.below(every);
    let mut notes = Vec::new();
    source(
        g,
        "Журнал Марата",
        None,
        &[F::No, F::Date, F::Due],
        &refs,
        &report,
        &mut |f, i| {
            if (i + offset).is_multiple_of(every) && matches!(f, F::Date | F::Due) {
                kinds.get((i / every + usize::from(f == F::Due)) % kinds.len()).copied().unwrap_or(Mess::TextDate)
            } else {
                Mess::Clean
            }
        },
        &mut notes,
    );
    template(g, &report, &recs, notes, &[]);
    let days = if g.level >= 4 {
        " В последнем столбце — сколько дней идёт поставка (срок − дата)."
    } else {
        ""
    };
    let prompt =
        format!("Марат вёл журнал как привык: часть дат — текстом, часть словами. Перенеси в «Отчёт» настоящими датами (все даты — 2026 год).{days}");
    Ok(finish(
        g,
        "Даты в один формат",
        prompt,
        "Текст «12.03.2026» → =ЗНАЧЕН(B4) или перепечатай. «12 марта» → набери 12.03.2026 или =ДАТА(2026;3;12).",
    ))
}

pub fn names(g: &mut Gen) -> Result<Task> {
    let n = size(g.level).max(5);
    let recs = records(g, n);
    let report = vec![F::No, F::Sup, F::Amount];
    let src = shuffled(g, &recs);
    let refs: Vec<&Rec> = src.iter().collect();
    let kinds: Vec<Mess> = match g.level {
        1 => vec![Mess::NameVariant],
        2 => vec![Mess::NameVariant, Mess::NameSpaces],
        _ => vec![Mess::NameVariant, Mess::NameCase, Mess::NameSpaces],
    };
    let dirty = 1 + usize::from(g.level) * 3 / 2;
    let offset = g.l.below(2);
    let mut notes = Vec::new();
    // The reference list: how names are spelled in the company.
    let mut book: Vec<&str> = recs.iter().map(|r| r.sup).collect();
    book.sort_unstable();
    book.dedup();
    g.setup.text(q("Справочник", 1, 0), "Поставщик (как в 1С)");
    g.setup.bold.push(q("Справочник", 1, 0));
    g.setup.widths.push(("'Справочник'!A:A".into(), 220.0));
    for (i, b) in book.iter().enumerate() {
        g.setup.input(q("Справочник", 2 + i as u32, 0), Cell::Text((*b).into()));
    }
    source(
        g,
        "Заказы (вручную)",
        None,
        &[F::No, F::Sup, F::Amount],
        &refs,
        &report,
        &mut |f, i| {
            let k = (i + offset) / 2;
            if f == F::Sup && (i + offset).is_multiple_of(2) && k < dirty {
                kinds.get(k % kinds.len()).copied().unwrap_or(Mess::Clean)
            } else {
                Mess::Clean
            }
        },
        &mut notes,
    );
    template(g, &report, &recs, notes, &[]);
    let prompt = "Заказы вводили разные люди, и поставщики записаны кто как. В «Отчёте» названия должны быть точно как в «Справочнике».".to_string();
    Ok(finish(
        g,
        "Названия по справочнику",
        prompt,
        "«Вектор ТОО», «ТОО «Вектор»» → «ТОО Вектор»: исправь руками; лишние пробелы убирает =СЖПРОБЕЛЫ(B4).",
    ))
}

pub fn lookup(g: &mut Gen) -> Result<Task> {
    let n = size(g.level).max(5);
    let recs = records(g, n);
    let report = vec![F::No, F::Art, F::Qty, F::Item, F::Price, F::Amount];
    // The price list: from level 3 the article is not the first column, as in many real lists.
    let sheet = "Прайс поставщика";
    let art_first = g.level <= 2;
    let pcols: [F; 3] = if art_first { [F::Art, F::Item, F::Price] } else { [F::Item, F::Art, F::Price] };
    let heads = ["Артикул", "Наименование", "Цена, тг"];
    for (c, f) in pcols.iter().enumerate() {
        let h = match f {
            F::Art => heads[0],
            F::Item => heads[1],
            _ => heads[2],
        };
        g.setup.text(q(sheet, 1, c as u32), h);
        g.setup.widths.push((format!("'{sheet}'!{0}:{0}", crate::sheet::col_name(c as u32)), if *f == F::Item { 200.0 } else { 100.0 }));
    }
    g.setup.bold.push(format!("'{sheet}'!A1:C1"));
    let mut order: Vec<usize> = (0..ITEMS.len()).collect();
    g.l.shuffle(&mut order);
    for (i, item) in order.iter().enumerate() {
        let r = Rec {
            no: String::new(),
            date: 0.0,
            due: 0.0,
            sup: "",
            item: *item,
            qty: 0.0,
            price: ITEMS.get(*item).map(|x| x.1 as f64).unwrap_or(0.0),
        };
        for (c, f) in pcols.iter().enumerate() {
            g.setup.input(q(sheet, 2 + i as u32, c as u32), f.truth(&r));
        }
    }
    g.setup.formats.push((format!("'{sheet}'!C2:C{}", ITEMS.len() + 1), "#,##0".into()));
    // A hand-typed article with a trailing space: the lookup misses it until it is cleaned.
    let bad = (g.level >= 2).then(|| g.l.below(n));
    let mut notes = Vec::new();
    if let Some(r) = bad.and_then(|i| recs.get(i)) {
        let why = format!("артикул «{} » в отчёте с пробелом на конце — поиск его не находит, почисти его", r.art());
        notes.push((r.no.clone(), 3, why.clone()));
        notes.push((r.no.clone(), 4, why));
    }
    template(g, &report, &recs, notes, &[F::No, F::Art, F::Qty]);
    if let Some(i) = bad {
        let cell = a1(2 + i as u32, 1);
        if let Some(entry) = g.setup.cells.iter_mut().find(|(c, ..)| *c == cell)
            && let Cell::Text(t) = &entry.1
        {
            entry.1 = Cell::Text(format!("{t} "));
        }
    }
    let how = if art_first { "" } else { " Артикул в прайсе — не первый столбец." };
    let prompt = format!("Подтяни в «Отчёт» товар и цену с листа «{sheet}» по артикулу и посчитай сумму (кол-во × цена).{how}");
    Ok(finish(
        g,
        "Цена с другого листа",
        prompt,
        "=ВПР(B2;'Прайс поставщика'!$A$2:$C$11;3;ЛОЖЬ). Если артикул не в первом столбце — =ИНДЕКС(…;ПОИСКПОЗ(B2;…;0)) или =ПРОСМОТРX.",
    ))
}

pub fn merge(g: &mut Gen) -> Result<Task> {
    let n = size(g.level).max(5) + 1;
    let recs = records(g, n);
    let report = vec![F::No, F::Date, F::Sup, F::Amount];
    let mut mixed = shuffled(g, &recs);
    let half = mixed.len() / 2;
    let second: Vec<Rec> = mixed.split_off(half);
    let mut first: Vec<&Rec> = mixed.iter().collect();
    let mut second: Vec<&Rec> = second.iter().collect();
    // From level 3 one order was entered by both managers: it goes into the report once.
    if g.level >= 3
        && let Some(dup) = second.first().copied()
    {
        first.push(dup);
    }
    let text_dates = g.level >= 4;
    let mut notes = Vec::new();
    source(g, "Айгерим", None, &[F::No, F::Date, F::Sup, F::Amount], &first, &report, &mut |_, _| Mess::Clean, &mut notes);
    second.reverse();
    source(
        g,
        "Марат",
        None,
        &[F::No, F::Sup, F::Amount, F::Date, F::Person],
        &second,
        &report,
        &mut |f, i| if text_dates && f == F::Date && i % 2 == 0 { Mess::TextDate } else { Mess::Clean },
        &mut notes,
    );
    template(g, &report, &recs, notes, &[]);
    let dup = if g.level >= 3 {
        " Один номер заказа — одна строка, даже если его записали оба."
    } else {
        ""
    };
    let prompt = format!(
        "Айгерим и Марат ведут заказы каждый на своём листе, столбцы у них в разном порядке. Собери все заказы в один список на листе «Отчёт».{dup}"
    );
    Ok(finish(
        g,
        "Два источника в один список",
        prompt,
        "Сначала скопируй заказы Айгерим, под ними — заказы Марата, переставив его столбцы под шапку отчёта.",
    ))
}

pub fn format(g: &mut Gen) -> Result<Task> {
    let n = size(g.level).max(6);
    let fresh = records(g, n);
    let recs = shuffled(g, &fresh);
    let fields = [F::No, F::Date, F::Sup, F::Item, F::Amount];
    g.setup.first_sheet = Some(REPORT.into());
    // Data are already in; the header is plain and nothing is formatted.
    for (c, f) in fields.iter().enumerate() {
        g.setup.text(a1(1, c as u32), f.header());
        g.setup.width(c as u32, if matches!(f, F::Sup | F::Item) { 190.0 } else { 110.0 });
    }
    for (i, r) in recs.iter().enumerate() {
        let row = 2 + i as u32;
        for (c, f) in fields.iter().enumerate() {
            match f.truth(r) {
                Cell::Date(d) => g.setup.put(a1(row, c as u32), Cell::Date(d), true),
                other => g.setup.input(a1(row, c as u32), other),
            }
        }
    }
    let last = 1 + n as u32;
    let amount = crate::sheet::col_name(4);
    let mut asks = vec!["сделай шапку жирной", "закрепи её, чтобы не уезжала при прокрутке"];
    g.check("Шапка жирная", Kind::Bold { range: "A1:E1".into() });
    g.check("Шапка закреплена", Kind::Frozen { rows: 1 });
    if g.level >= 2 {
        asks.push("покажи суммы с разделителем тысяч");
        g.check("Формат сумм", Kind::NumFmt { range: format!("{amount}2:{amount}{last}"), contains: "#,##0".into() });
    }
    if g.level >= 3 {
        asks.push("отсортируй заказы по дате, от ранних к поздним (всю таблицу целиком)");
        g.check("Сортировка по дате", Kind::Sorted { first: 2, last, cols: (0, 4), key: 1, descending: false });
    }
    if g.level >= 4 {
        let total: f64 = recs.iter().map(Rec::amount).sum();
        asks.push("под таблицей добавь строку «Итого» с суммой формулой");
        g.check("Итого", Kind::Expected { cell: format!("{amount}{}", last + 1), expected: Expectation::Number(total), formula: true });
        g.check("Подпись «Итого»", Kind::Value { cell: a1(last + 1, 0), expected: Cell::Text("Итого".into()) });
    }
    let prompt = format!("Андрей: «Отчёт перед отправкой приведи в порядок»: {}.", asks.join(", "));
    let mut task = finish(
        g,
        "Оформление отчёта",
        prompt,
        "Жирный — Ctrl+B. Закрепить — Вид → Закрепить верхнюю строку. Разделитель — Ctrl+1 → Числовой. Сортировка — Данные → Сортировка (выделив всю таблицу).",
    );
    task.focus = "A1".into();
    Ok(task)
}

pub fn report(g: &mut Gen) -> Result<Task> {
    let level = g.level;
    let n = size(level) + 2;
    let recs = records(g, n);
    let mut report = vec![F::No, F::Date, F::Sup, F::Item, F::Qty, F::Price, F::Amount];
    if level >= 4 {
        report.push(F::Due);
    }
    let mut mixed = shuffled(g, &recs);
    let half = mixed.len() / 2;
    let hand_recs: Vec<Rec> = mixed.split_off(half);
    let export: Vec<&Rec> = mixed.iter().collect();
    let mut hand: Vec<&Rec> = hand_recs.iter().collect();
    if level >= 3
        && let Some(dup) = export.first().copied()
    {
        hand.insert(hand.len() / 2, dup);
    }
    // From level 3 the sources hold the article, and prices live on the supplier's list.
    let priced = level <= 2;
    let base = if priced { vec![F::No, F::Date, F::Sup, F::Item, F::Qty, F::Price] } else { vec![F::No, F::Date, F::Sup, F::Art, F::Item, F::Qty] };
    let mut export_fields = base.clone();
    export_fields.push(F::Store);
    if level >= 4 {
        export_fields.push(F::Due);
    }
    let mut hand_fields = base.clone();
    if level >= 4 {
        hand_fields.push(F::Due);
    }
    let mut notes = Vec::new();
    let title = (level >= 2).then_some("Выгрузка из 1С, март 2026");
    source(g, "Выгрузка 1С", title, &export_fields, &export, &report, &mut |f, _| if f == F::Date { Mess::IsoDate } else { Mess::Clean }, &mut notes);
    let date_kinds = [Mess::TextDate, Mess::WordDate, Mess::ShortTextDate];
    source(
        g,
        "От Марата",
        None,
        &hand_fields,
        &hand,
        &report,
        &mut |f, i| match f {
            F::Sup if i % 3 == 0 => Mess::NameVariant,
            F::Sup if level >= 3 && i % 3 == 1 => Mess::NameCase,
            F::Date if i % 2 == 1 => date_kinds.get((i / 2) % date_kinds.len()).copied().unwrap_or(Mess::TextDate),
            F::Qty if level >= 2 && i % 3 == 2 => Mess::NumUnit,
            F::Price if level >= 2 && i % 4 == 0 => Mess::NumUnit,
            _ => Mess::Clean,
        },
        &mut notes,
    );
    if !priced {
        let sheet = "Прайс";
        for (c, h) in ["Наименование", "Артикул", "Цена, тг"].iter().enumerate() {
            g.setup.text(q(sheet, 1, c as u32), *h);
        }
        g.setup.bold.push(format!("'{sheet}'!A1:C1"));
        g.setup.widths.push((format!("'{sheet}'!A:A"), 200.0));
        for (i, (name, price)) in ITEMS.iter().enumerate() {
            g.setup.input(q(sheet, 2 + i as u32, 0), Cell::Text((*name).into()));
            g.setup.input(q(sheet, 2 + i as u32, 1), Cell::Text(format!("OM-{}", 101 + i)));
            g.setup.input(q(sheet, 2 + i as u32, 2), Cell::Num(*price as f64));
        }
    }
    let mut book: Vec<&str> = recs.iter().map(|r| r.sup).collect();
    book.sort_unstable();
    book.dedup();
    g.setup.text(q("Справочник", 1, 0), "Поставщик (как в 1С)");
    g.setup.bold.push(q("Справочник", 1, 0));
    g.setup.widths.push(("'Справочник'!A:A".into(), 220.0));
    for (i, b) in book.iter().enumerate() {
        g.setup.input(q("Справочник", 2 + i as u32, 0), Cell::Text((*b).into()));
    }
    template(g, &report, &recs, notes, &[]);
    let mut asks = Vec::new();
    if level >= 4 {
        let last = 1 + n as u32;
        let total: f64 = recs.iter().map(Rec::amount).sum();
        // The template's header comes plain at this level: formatting is part of the job.
        g.setup.bold.retain(|b| b.contains('!'));
        g.check("Шапка жирная", Kind::Bold { range: format!("A1:{}", a1(1, report.len() as u32 - 1)) });
        g.check("Шапка закреплена", Kind::Frozen { rows: 1 });
        g.check("Итого", Kind::Expected { cell: a1(last + 1, 6), expected: Expectation::Number(total), formula: true });
        asks.push(format!(" Оформи: жирная закреплённая шапка, под таблицей строка «Итого» с суммой формулой в {}.", a1(last + 1, 6)));
    }
    let price = if priced { "" } else { " Цены — с листа «Прайс» по артикулу." };
    let dup = if level >= 3 { " Один номер заказа — одна строка." } else { "" };
    let prompt = format!(
        "Андрей: «Собери в «Отчёт» все мартовские заказы: часть в выгрузке из 1С, часть Марат вёл вручную. Названия — как в «Справочнике», даты и числа — настоящие, сумма = кол-во × цена.{price}{dup}»{}",
        asks.concat()
    );
    Ok(finish(
        g,
        "Отчёт целиком",
        prompt,
        "Порядок: перенеси строки из обоих листов под шапку → почини даты, числа и названия → досчитай сумму → проверь, что номеров столько же, сколько в источниках.",
    ))
}

/// A source sheet laid out exactly like the report: same headers, same order. The plain
/// case of «add the months under each other».
fn plain(g: &mut Gen, sheet: &str, fields: &[F], recs: &[&Rec]) {
    for (c, f) in fields.iter().enumerate() {
        g.setup.text(q(sheet, 1, c as u32), f.header());
        let w = if matches!(f, F::Sup | F::Item) { 180.0 } else { 105.0 };
        g.setup.widths.push((format!("'{sheet}'!{0}:{0}", crate::sheet::col_name(c as u32)), w));
    }
    g.setup.bold.push(format!("'{sheet}'!{}:{}", a1(1, 0), a1(1, fields.len() as u32 - 1)));
    for (i, r) in recs.iter().enumerate() {
        for (c, f) in fields.iter().enumerate() {
            let at = q(sheet, 2 + i as u32, c as u32);
            let cell = match f.truth(r) {
                Cell::Date(n) => {
                    g.setup.formats.push((at.clone(), "dd.mm.yyyy".into()));
                    Cell::Num(n)
                }
                other => other,
            };
            g.setup.input(at, cell);
        }
    }
}

/// Several monthly exports, one pivot. A pivot reads one list, so the months go under each
/// other first (one header, a «Месяц» column saying where each row came from), then the
/// pivot is built from that list. Power Query does the same append automatically; the
/// learner is told its name, the engine has none.
pub fn pivot(g: &mut Gen) -> Result<Task> {
    use gridcraft_engine::model::PivotFunc;
    let level = g.level;
    let per = match level {
        1 => 3,
        2 => 4,
        3 => 5,
        _ => 6,
    };
    let mut recs = records(g, per * 3);
    // One generator for the quarter keeps order numbers unique; each third moves to its month.
    for (k, chunk) in recs.chunks_mut(per).enumerate() {
        let shift = chunk.first().map_or(0.0, |r| date(2026, 1 + k as i64, 3) - r.date);
        for r in chunk.iter_mut() {
            r.date += shift;
            r.due += shift;
        }
    }
    let fields = [F::No, F::Date, F::Sup, F::Amount];
    // «Месяц» last: a month's rows paste whole, the month is typed to their right.
    let report = vec![F::No, F::Date, F::Sup, F::Amount, F::Month];
    let mut notes = Vec::new();
    for (k, chunk) in recs.chunks(per).enumerate() {
        let sheet = MONTH_NAMES.get(k).copied().unwrap_or("Март");
        let rows: Vec<&Rec> = chunk.iter().collect();
        // From level 3 each month comes from a different hand: other column order and names.
        if level <= 2 || k == 0 {
            plain(g, sheet, &fields, &rows);
        } else {
            let names = level >= 4 && k == 2;
            source(
                g,
                sheet,
                None,
                &fields,
                &rows,
                &report,
                &mut |f, i| if names && f == F::Sup && i % 2 == 0 { Mess::NameVariant } else { Mess::Clean },
                &mut notes,
            );
        }
    }
    if level >= 4 {
        let mut book: Vec<&str> = recs.iter().map(|r| r.sup).collect();
        book.sort_unstable();
        book.dedup();
        g.setup.text(q("Справочник", 1, 0), "Поставщик (как в ERP)");
        g.setup.bold.push(q("Справочник", 1, 0));
        g.setup.widths.push(("'Справочник'!A:A".into(), 220.0));
        for (i, b) in book.iter().enumerate() {
            g.setup.input(q("Справочник", 2 + i as u32, 0), Cell::Text((*b).into()));
        }
    }
    template(g, &report, &recs, notes, &[]);
    let (sup, month, amount) = (F::Sup.header().to_string(), F::Month.header().to_string(), F::Amount.header().to_string());
    let (rows, columns, func) = match level {
        1 => (vec![sup.clone()], vec![], PivotFunc::Sum),
        2 | 3 => (vec![sup.clone()], vec![month.clone()], PivotFunc::Sum),
        _ => (vec![sup.clone()], vec![month.clone()], PivotFunc::Count),
    };
    g.check(
        "Сводная по списку",
        Kind::Pivot {
            sheet: String::new(),
            source: format!("'{REPORT}'!A1:{}", a1(recs.len() as u32 + 1, report.len() as u32 - 1)),
            rows,
            columns,
            values: vec![(amount, func)],
        },
    );
    let collect = "Сводная строится из ОДНОГО списка. Сначала перенеси на лист «Отчёт» заказы всех трёх месяцев друг под другом (шапка одна, без пустых строк) и заполни «Месяц» названием листа, откуда строка.";
    let prompt = match level {
        1 => format!(
            "Андрей: «Сколько мы потратили у каждого поставщика за квартал?» Заказы лежат на листах «Январь», «Февраль», «Март». {collect} Потом встань в список → Вставка → Сводная таблица: «Поставщик» в строки, «Сумма» в значения."
        ),
        2 => format!(
            "Андрей: «Покажи траты по поставщикам помесячно». {collect} Потом сводная: «Поставщик» в строках, «Месяц» в столбцах, «Сумма» в значениях."
        ),
        3 => "Андрей: «К планёрке: траты по каждому поставщику помесячно, сводной». Январь из ERP, а февраль и март вели другие люди: столбцы стоят иначе и называются по-своему. Собери всё на «Отчёт» под его шапку и построй сводную.".to_string(),
        _ => "Андрей: «Сколько заказов мы сделали у каждого поставщика в каждом месяце? Нужна сводная». Месяцы вели разные люди; в марте названия поставщиков записаны вразнобой - в отчёте пиши как в «Справочнике».".to_string(),
    };
    let mut task = finish(
        g,
        "Сводная из нескольких выгрузок",
        prompt,
        "На листе «Январь» выдели строки с данными (без шапки) → Ctrl+C → на «Отчёте» встань в A2 → Ctrl+V. Справа в столбце «Месяц» впиши «Январь» и протяни до конца января. Ниже так же февраль и март. Потом встань в A2 → Вставка → Сводная таблица.",
    );
    task.goal_seconds = 120.0 + 60.0 * f32::from(level);
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::evaluate;
    use crate::sheet::{self};
    use crate::{Spec, Task};
    use gridcraft_engine::core::{CellRef, Value};

    const SKILLS: &[&str] = &["asm.columns", "asm.numbers", "asm.dates", "asm.names", "asm.lookup", "asm.merge", "asm.format", "asm.report", "asm.pivot"];

    fn table(task: &Task) -> Option<(Vec<String>, usize)> {
        task.checks.iter().find_map(|c| match &c.kind {
            Kind::Table { headers, rows, .. } => Some((headers.clone(), rows.len())),
            _ => None,
        })
    }

    #[test]
    fn every_level_builds_a_real_multi_sheet_book_without_overlaps() {
        for skill in SKILLS {
            for level in 1..=4u8 {
                for seed in 1..=12u64 {
                    let spec = Spec::new(skill, level, seed * 31);
                    let task = Task::generate(&spec).unwrap_or_else(|e| panic!("{skill} L{level}: {e}"));
                    let mut seen = std::collections::BTreeSet::new();
                    for (cell, ..) in &task.setup.cells {
                        assert!(seen.insert(cell.clone()), "{skill} L{level} s{seed}: {cell} written twice");
                    }
                    let s = task.session().unwrap_or_else(|e| panic!("{skill} L{level}: {e}"));
                    let wb = &s.doc().expect("doc").wb;
                    assert_eq!(wb.sheets.first().map(|sh| sh.name.as_str()), Some(REPORT));
                    if *skill != "asm.format" {
                        assert!(wb.sheets.len() >= 2, "{skill}: sources live on their own sheets");
                    }
                    assert!(!evaluate(&task, &s, &[]).expect("evaluate").passed(), "{skill} L{level} passes untouched");
                }
            }
        }
    }

    /// The field a source header stands for (aliases are unique across fields).
    fn field_of(header: &str) -> Option<F> {
        [F::No, F::Date, F::Due, F::Sup, F::Item, F::Art, F::Qty, F::Price, F::Amount, F::Month, F::Days, F::Store, F::Person]
            .into_iter()
            .find(|f| f.aliases().contains(&header) || f.header() == header)
    }

    /// What a hurried learner does: copy every source row as it is under the matching header.
    fn copy_as_is(task: &Task, s: &mut gridcraft_engine::Session) {
        let (headers, _) = table(task).expect("table");
        let report: Vec<F> = headers.iter().filter_map(|h| field_of(h)).collect();
        let wb = s.doc().expect("doc").wb.clone();
        let mut out = 2u32;
        for sh in wb.sheets.iter().skip(1) {
            if sh.name == "Справочник" || sh.name.starts_with("Прайс") {
                continue;
            }
            let header_row = (0..4u32).find(|r| field_of(sh.value(CellRef::new(*r, 0)).as_text().unwrap_or("")).is_some()).expect("header");
            let map: Vec<Option<usize>> = (0..12u32)
                .map(|c| field_of(sh.value(CellRef::new(header_row, c)).as_text().unwrap_or("")).and_then(|f| report.iter().position(|x| *x == f)))
                .collect();
            let mut r = header_row + 1;
            while !matches!(sh.value(CellRef::new(r, 0)), Value::Empty) {
                for (c, to) in map.iter().enumerate() {
                    let Some(to) = to else { continue };
                    let v = sh.value(CellRef::new(r, c as u32));
                    let cell = match v {
                        Value::Number(n) => Cell::Num(n),
                        Value::Text(t) => Cell::Text(format!("'{t}")),
                        _ => continue,
                    };
                    sheet::set(s, &a1(out, *to as u32), &cell).expect("copy");
                }
                out += 1;
                r += 1;
            }
        }
    }

    fn feedback(task: &Task, s: &gridcraft_engine::Session) -> (bool, String) {
        let e = evaluate(task, s, &[]).expect("evaluate");
        (e.passed(), e.outcomes.iter().map(|o| o.feedback.clone()).collect::<Vec<_>>().join(" | "))
    }

    #[test]
    fn copying_as_is_works_for_clean_sources_and_names_each_dirty_spot() {
        for seed in [4u64, 51, 700] {
            for level in 1..=4u8 {
                let task = Task::generate(&Spec::new("asm.columns", level, seed)).expect("task");
                let mut s = task.session().expect("session");
                copy_as_is(&task, &mut s);
                let (ok, fb) = feedback(&task, &s);
                assert!(ok, "columns L{level} s{seed}: {fb}");

                let task = Task::generate(&Spec::new("asm.dates", level, seed)).expect("task");
                let mut s = task.session().expect("session");
                copy_as_is(&task, &mut s);
                let (ok, fb) = feedback(&task, &s);
                assert!(!ok && fb.contains("настоящая дата") && fb.contains("«Журнал Марата», строка"), "dates L{level} s{seed}: {fb}");

                let task = Task::generate(&Spec::new("asm.numbers", level, seed)).expect("task");
                let mut s = task.session().expect("session");
                copy_as_is(&task, &mut s);
                let (ok, fb) = feedback(&task, &s);
                assert!(!ok && fb.contains("нужно число"), "numbers L{level} s{seed}: {fb}");

                let task = Task::generate(&Spec::new("asm.names", level, seed)).expect("task");
                let mut s = task.session().expect("session");
                copy_as_is(&task, &mut s);
                let (ok, fb) = feedback(&task, &s);
                assert!(!ok && fb.contains("справочник"), "names L{level} s{seed}: {fb}");
            }
            // Both managers entered one order: copying both lists repeats it.
            let task = Task::generate(&Spec::new("asm.merge", 3, seed)).expect("task");
            let mut s = task.session().expect("session");
            copy_as_is(&task, &mut s);
            let (ok, fb) = feedback(&task, &s);
            assert!(!ok && fb.contains("записан 2 раза"), "merge s{seed}: {fb}");
        }
    }

    #[test]
    fn day_month_swaps_and_extra_rows_are_named() {
        let task = Task::generate(&Spec::new("asm.dates", 1, 9)).expect("task");
        let Some(Kind::Table { rows, .. }) = task.checks.first().map(|c| c.kind.clone()) else { panic!("table") };
        let mut s = task.session().expect("session");
        for (i, row) in rows.iter().enumerate() {
            for (c, want) in row.iter().enumerate() {
                let cell = match want {
                    Expectation::Number(n) => Cell::Num(*n),
                    Expectation::Text(t) => Cell::Text(t.clone()),
                    _ => continue,
                };
                sheet::set(&mut s, &a1(2 + i as u32, c as u32), &cell).expect("set");
            }
        }
        assert!(feedback(&task, &s).0);
        // A date read month-first.
        let swapped_row = rows
            .iter()
            .position(|r| matches!(r.get(1), Some(Expectation::Number(n)) if crate::sheet::ymd(*n).is_some_and(|(_, m, d)| d <= 12 && d != m)));
        if let Some(i) = swapped_row {
            let Some(Expectation::Number(n)) = rows[i].get(1) else { panic!() };
            let (y, m, d) = crate::sheet::ymd(*n).expect("ymd");
            sheet::set(&mut s, &a1(2 + i as u32, 1), &Cell::Num(date(y, d, m))).expect("set");
            let (ok, fb) = feedback(&task, &s);
            assert!(!ok && fb.contains("перепутаны день и месяц"), "{fb}");
            sheet::set(&mut s, &a1(2 + i as u32, 1), &Cell::Num(*n)).expect("set");
        }
        // A pasted header row is not a source row.
        let below = 2 + rows.len() as u32;
        sheet::set(&mut s, &a1(below, 0), &Cell::Text("Номер".into())).expect("set");
        let (ok, fb) = feedback(&task, &s);
        assert!(!ok && fb.contains("такой строки в источниках нет"), "{fb}");
        // «Итого» rows are allowed under the data.
        sheet::set(&mut s, &a1(below, 0), &Cell::Text("Итого".into())).expect("set");
        assert!(feedback(&task, &s).0);
    }

    #[test]
    fn a_cross_sheet_lookup_fills_the_report_once_the_article_is_cleaned() {
        for level in 1..=4u8 {
            let task = Task::generate(&Spec::new("asm.lookup", level, 12)).expect("task");
            let Some((_, n)) = table(&task) else { panic!("table") };
            let mut s = task.session().expect("session");
            let wb = s.doc().expect("doc").wb.clone();
            let price = wb.sheets.get(1).expect("price sheet");
            let art_col = (0..3u32).find(|c| price.value(CellRef::new(0, *c)).as_text() == Some("Артикул")).expect("article column");
            let col = |c: u32| crate::sheet::col_name(c);
            let (a, p, nm) = (col(art_col), col(2), col(if art_col == 0 { 1 } else { 0 }));
            let range = |c: &str| format!("'Прайс поставщика'!${c}$2:${c}$11");
            for r in 2..2 + n as u32 {
                let key = format!("TRIM(B{r})");
                sheet::set(&mut s, &format!("D{r}"), &Cell::Formula(format!("=INDEX({},MATCH({key},{},0))", range(&nm), range(&a)))).expect("item");
                sheet::set(&mut s, &format!("E{r}"), &Cell::Formula(format!("=INDEX({},MATCH({key},{},0))", range(&p), range(&a)))).expect("price");
                sheet::set(&mut s, &format!("F{r}"), &Cell::Formula(format!("=C{r}*E{r}"))).expect("amount");
            }
            let (ok, fb) = feedback(&task, &s);
            assert!(ok, "lookup L{level}: {fb}");
            // Without TRIM the spaced article is not found, and the miss says why.
            if level >= 2 {
                for r in 2..2 + n as u32 {
                    sheet::set(&mut s, &format!("E{r}"), &Cell::Formula(format!("=INDEX({},MATCH(B{r},{},0))", range(&p), range(&a))))
                        .expect("price");
                }
                let (ok, fb) = feedback(&task, &s);
                assert!(!ok && fb.contains("с пробелом на конце"), "lookup L{level}: {fb}");
            }
        }
    }

    /// The learner's path: months under one header, «Месяц» filled, Insert → PivotTable from
    /// a cell of the list (it lands on a new sheet in front), fields ticked in the pane.
    #[test]
    fn months_appended_then_a_pivot_from_the_ribbon_passes_and_shortcuts_are_named() {
        use serde_json::json;
        for level in 1..=4u8 {
            for seed in [3u64, 40, 777] {
                let task = Task::generate(&Spec::new("asm.pivot", level, seed)).expect("task");
                let Some(Kind::Table { rows, headers, .. }) = task.checks.first().map(|c| c.kind.clone()) else { panic!("table") };
                let Some(Kind::Pivot { rows: prow, columns, values, .. }) = task.checks.get(1).map(|c| c.kind.clone()) else { panic!("pivot") };
                assert_eq!(headers, ["№ заказа", "Дата", "Поставщик", "Сумма", "Месяц"]);
                let months: std::collections::BTreeSet<String> =
                    rows.iter().filter_map(|r| if let Some(Expectation::Text(m)) = r.get(4) { Some(m.clone()) } else { None }).collect();
                assert_eq!(months.len(), 3, "three months");
                let mut s = task.session().expect("session");
                // Only January copied: the list is short, and so is the pivot's source.
                let jan: Vec<_> = rows.iter().filter(|r| r.get(4) == Some(&Expectation::Text("Январь".into()))).cloned().collect();
                let put = |s: &mut gridcraft_engine::Session, list: &[Vec<Expectation>]| {
                    for (i, row) in list.iter().enumerate() {
                        for (c, want) in row.iter().enumerate() {
                            let cell = match want {
                                Expectation::Number(n) => Cell::Num(*n),
                                Expectation::Text(t) => Cell::Text(t.clone()),
                                _ => continue,
                            };
                            sheet::set(s, &a1(2 + i as u32, c as u32), &cell).expect("set");
                        }
                    }
                };
                put(&mut s, &jan);
                let build = |s: &mut gridcraft_engine::Session| {
                    s.execute("selection.set", json!({"range": format!("'{REPORT}'!A2")})).expect("select");
                    let r = s.execute("insert.pivotTable", json!({})).expect("pivot");
                    let name = r["name"].as_str().map(str::to_string).unwrap_or_else(|| "PivotTable1".into());
                    for f in prow.iter().chain(&columns) {
                        s.execute("pivot.addField", json!({"pivot": name, "field": f, "area": "rows"})).expect("rows");
                    }
                    for f in &columns {
                        s.execute("pivot.moveField", json!({"pivot": name, "field": f, "from": "rows", "to": "columns"})).expect("cols");
                    }
                    let (amount, func) = values[0].clone();
                    s.execute("pivot.addField", json!({"pivot": name, "field": amount, "area": "values"})).expect("values");
                    if func != gridcraft_engine::model::PivotFunc::Sum {
                        s.execute("pivot.valueSettings", json!({"pivot": name, "field": amount, "func": "count"})).expect("count");
                    }
                };
                build(&mut s);
                let (ok, fb) = feedback(&task, &s);
                assert!(!ok && fb.contains("нет строки"), "L{level} s{seed} January only: {fb}");
                // The whole quarter: the report is still found behind the new pivot sheet.
                let mut s = task.session().expect("session");
                put(&mut s, &rows);
                build(&mut s);
                assert_ne!(s.doc().expect("doc").wb.sheets.first().map(|sh| sh.name.as_str()), Some(REPORT), "pivot sheet in front");
                let (ok, fb) = feedback(&task, &s);
                assert!(ok, "L{level} s{seed}: {fb}");
            }
        }
    }
}
