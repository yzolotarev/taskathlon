//! Live data: the function just practised, on a supply register someone typed by hand.
//!
//! A clean drill proves the syntax; work data is messier. A live task asks the same kind of
//! question about a hand-kept register with real entry errors: stray spaces, «5 шт» and
//! «12500 тг» in number cells, a number stored as text, a subtotal row, a blank row and an
//! order typed twice. Dirt grows one kind per step (tier 1..=4) and the answer cell fades
//! from highlighted, to labelled, to "lay it out yourself".
//!
//! The answer is an independent oracle over the clean truth. Every subset of untreated dirt
//! also has its wrong answer precomputed, so a miss names the trap that produced it.

use crate::check::{Check, Expectation, Kind};
use crate::rng::Rng;
use crate::sheet::{Cell, Setup, a1, date, num_text};
use crate::{Mode, Result, Spec, Task};

pub const PREFIX: &str = "live.";
pub const TIERS: u8 = 4;
/// The label a learner writes next to a self-placed answer (tiers 3–4).
pub const ANSWER_LABEL: &str = "Ответ";

pub(super) const SUPPLIERS: &[&str] = &["ТОО Вектор", "ТОО АлматыЭлектро", "ТОО СтепьКабель", "ТОО КазРадиоКомплект", "ТОО Тянь-Шань Трейд"];
/// Never a target, so its name never needs a variant.
pub(super) const SIDE_SUPPLIER: &str = "ИП Ахметов";
const STATUSES: [&str; 3] = ["Получен", "В пути", "Заказан"];
pub(super) const ITEMS: &[(&str, i64)] = &[
    ("Датчик температуры LoRa", 18_500),
    ("Счётчик импульсов", 24_000),
    ("Антенна 868 МГц", 6_200),
    ("Кабель RG-58, м", 450),
    ("Батарея ER14505", 1_900),
    ("Корпус IP67", 3_400),
    ("Плата контроллера", 42_000),
    ("Блок питания 12 В", 7_800),
    ("Модем NB-IoT", 15_600),
    ("Крепёж DIN-рейка", 650),
];
const COMMENTS: &[&str] = &["", "", "", "срочно", "звонил, обещали к пятнице", "частичная поставка", "счёт в работе", "уточнить цену", ""];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dirt {
    NameSpaces,
    NameVariant,
    StatusVariant,
    AmountText,
    AmountUnit,
    QtyUnit,
    PriceUnit,
    Subtotal,
    BlankRow,
    Duplicate,
    ArticleSpace,
    ArticleNoDash,
}

const ALL_DIRT: [Dirt; 12] = [
    Dirt::NameSpaces,
    Dirt::NameVariant,
    Dirt::StatusVariant,
    Dirt::AmountText,
    Dirt::AmountUnit,
    Dirt::QtyUnit,
    Dirt::PriceUnit,
    Dirt::Subtotal,
    Dirt::BlankRow,
    Dirt::Duplicate,
    Dirt::ArticleSpace,
    Dirt::ArticleNoDash,
];

impl Dirt {
    fn bit(self) -> u16 {
        1 << ALL_DIRT.iter().position(|d| *d == self).unwrap_or(0)
    }
    /// How to deal with it, without naming rows (hint 2).
    fn tool(self) -> &'static str {
        match self {
            Dirt::NameSpaces => "лишние пробелы в названии убирает СЖПРОБЕЛЫ, или исправь ячейку вручную",
            Dirt::NameVariant => "одно и то же название записано по-разному: исправь вручную или возьми условие со звёздочкой, например \"*Вектор*\"",
            Dirt::StatusVariant => "статус записан по-разному: условие со звёздочкой (\"Получ*\") или ручная правка",
            Dirt::AmountText => "число, записанное текстом, прижато к левому краю: перепечатай его или возьми =ЗНАЧЕН(ячейка)",
            Dirt::AmountUnit | Dirt::PriceUnit => "«тг» в ячейке делает число текстом: убери вручную или =ЗНАЧЕН(ПОДСТАВИТЬ(ячейка;\" тг\";\"\"))",
            Dirt::QtyUnit => "«шт» в ячейке делает число текстом: убери вручную или =ЗНАЧЕН(ПОДСТАВИТЬ(ячейка;\" шт\";\"\"))",
            Dirt::Subtotal => "строка «Итого…» посреди данных — не заказ: удали её или не включай в расчёт",
            Dirt::BlankRow => "пустая строка обрывает Ctrl+Shift+↓ и автосумму: проверь, где на самом деле кончается таблица",
            Dirt::Duplicate => "один номер заказа — один заказ: найди повтор и удали его",
            Dirt::ArticleSpace | Dirt::ArticleNoDash => {
                "артикул должен совпадать с каталогом символ в символ: исправь вручную (пробелы убирает СЖПРОБЕЛЫ)"
            }
        }
    }
    /// Which columns to look at (hint 1).
    fn column(self) -> &'static str {
        match self {
            Dirt::NameSpaces | Dirt::NameVariant => "«Поставщик»",
            Dirt::StatusVariant => "«Статус»",
            Dirt::AmountText | Dirt::AmountUnit => "«Сумма»",
            Dirt::QtyUnit => "«Кол-во»",
            Dirt::PriceUnit => "«Цена»",
            Dirt::Subtotal | Dirt::BlankRow | Dirt::Duplicate => "«№ заказа»",
            Dirt::ArticleSpace | Dirt::ArticleNoDash => "«Артикул»",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Question {
    Total,
    Average,
    Max,
    Orders,
    SumBy,
    CountBy,
    CountAbove,
    SumBy2,
    CountBy2,
    AverageBy,
    LineTotal,
    CatalogTotal,
}

/// The question a skill answers on live data, and the dirt that can hide in it, in teaching
/// order: tier 1 has only the first kind, tier 2 the first two, later tiers add the rest.
/// Spotting errors is a skill of its own, so the first kind is always one you can see
/// (a stray «тг», a subtotal row, a name written differently); invisible ones (stray spaces,
/// a number stored as text) come later.
fn question(skill: &str) -> Option<(Question, &'static [Dirt])> {
    use Dirt::*;
    Some(match skill {
        "fn.sum" => (Question::Total, &[Subtotal, AmountUnit, AmountText, BlankRow, Duplicate]),
        "fn.average" => (Question::Average, &[Subtotal, AmountUnit, AmountText, Duplicate]),
        "fn.minmax" => (Question::Max, &[Subtotal, AmountText, BlankRow]),
        "fn.count" => (Question::Orders, &[Subtotal, Duplicate, BlankRow]),
        "cond.sumif" => (Question::SumBy, &[NameVariant, NameSpaces, AmountText, BlankRow, Duplicate]),
        "cond.countif" => (Question::CountBy, &[NameVariant, NameSpaces, Duplicate, BlankRow]),
        "cond.compare" => (Question::CountAbove, &[Subtotal, AmountUnit, AmountText, Duplicate]),
        "cond.sumifs" => (Question::SumBy2, &[StatusVariant, NameVariant, NameSpaces, AmountText, Duplicate]),
        "cond.countifs" => (Question::CountBy2, &[StatusVariant, NameVariant, NameSpaces, Duplicate]),
        "cond.averageif" => (Question::AverageBy, &[NameVariant, NameSpaces, AmountText, Duplicate]),
        "start.mul" | "refs.fill" => (Question::LineTotal, &[QtyUnit, PriceUnit, BlankRow, Duplicate]),
        "lookup.vlookup" | "lookup.xlookup" | "lookup.index" => (Question::CatalogTotal, &[ArticleNoDash, ArticleSpace, QtyUnit, BlankRow]),
        _ => return None,
    })
}

/// Skills that continue on live data after a passed drill.
pub fn supports(skill: &str) -> bool {
    question(skill).is_some()
}

/// The practised skill behind a live task id.
pub fn base(id: &str) -> Option<&str> {
    id.strip_prefix(PREFIX).filter(|s| supports(s))
}

/// A live spec: `level` is the dirt tier; the low byte of `data` is the first sheet row,
/// so the register can open below a finished drill in the same book.
pub fn spec(skill: &str, tier: u8, seed: u64, top: u32) -> Spec {
    let mut s = Spec::new(&format!("{PREFIX}{skill}"), tier.clamp(1, TIERS), seed);
    s.data = (s.data & !0xFF) | u64::from(top.clamp(1, 200));
    s
}

pub fn with_top(spec: &Spec, top: u32) -> Spec {
    let mut s = spec.clone();
    s.data = (s.data & !0xFF) | u64::from(top.clamp(1, 200));
    s
}

fn top_of(spec: &Spec) -> u32 {
    ((spec.data & 0xFF) as u32).max(1)
}

#[derive(Clone, Debug)]
struct Order {
    no: String,
    date: f64,
    sup: usize,
    item: usize,
    qty: f64,
    price: f64,
    status: usize,
    comment: &'static str,
    dirt: Vec<Dirt>,
}

impl Order {
    fn amount(&self) -> f64 {
        self.qty * self.price
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Line {
    Order(usize),
    Dup(usize),
    Subtotal(f64),
    Blank,
}

/// What a straightforward formula produces when some dirt is left in place.
#[derive(Clone, Debug, PartialEq)]
enum Naive {
    Num(f64),
    Err(&'static str),
}

struct Register {
    q: Question,
    orders: Vec<Order>,
    lines: Vec<Line>,
    /// Supplier names; index 0 is the question's supplier, the last is the side supplier.
    suppliers: Vec<&'static str>,
    threshold: f64,
    /// Catalog price per item index (lookup questions).
    catalog: Vec<f64>,
    present: Vec<Dirt>,
}

impl Register {
    fn relevant(&self, o: &Order) -> bool {
        match self.q {
            Question::SumBy | Question::CountBy | Question::AverageBy => o.sup == 0,
            Question::SumBy2 | Question::CountBy2 => o.sup == 0 && o.status == 0,
            Question::CountAbove => o.amount() > self.threshold,
            _ => true,
        }
    }

    fn price_of(&self, o: &Order) -> f64 {
        if self.q == Question::CatalogTotal { self.catalog.get(o.item).copied().unwrap_or(0.0) } else { o.price }
    }

    /// The answer with dirt in `untreated` left as it is (0 = everything handled).
    fn measure(&self, untreated: u16) -> Vec<Naive> {
        let left = |d: Dirt| untreated & d.bit() != 0;
        struct Row {
            amount: Option<f64>,
            sup: bool,
            status: bool,
            cost: std::result::Result<f64, &'static str>,
        }
        let mut rows = Vec::new();
        for line in &self.lines {
            match *line {
                Line::Blank if left(Dirt::BlankRow) => break,
                Line::Blank => {}
                Line::Subtotal(v) if left(Dirt::Subtotal) => rows.push(Row { amount: Some(v), sup: false, status: false, cost: Ok(v) }),
                Line::Subtotal(_) => {}
                Line::Dup(_) if !left(Dirt::Duplicate) => {}
                Line::Order(i) | Line::Dup(i) => {
                    let Some(o) = self.orders.get(i) else { continue };
                    let dirty = |d: Dirt| matches!(line, Line::Order(_)) && o.dirt.contains(&d) && left(d);
                    let amount = (!dirty(Dirt::AmountText) && !dirty(Dirt::AmountUnit)).then(|| o.amount());
                    let sup = o.sup == 0 && !dirty(Dirt::NameSpaces) && !dirty(Dirt::NameVariant);
                    let status = o.status == 0 && !dirty(Dirt::StatusVariant);
                    let cost = if dirty(Dirt::ArticleSpace) || dirty(Dirt::ArticleNoDash) {
                        Err("#N/A")
                    } else if dirty(Dirt::QtyUnit) || dirty(Dirt::PriceUnit) {
                        Err("#VALUE!")
                    } else {
                        Ok(o.qty * self.price_of(o))
                    };
                    rows.push(Row { amount, sup, status, cost });
                }
            }
        }
        let nums = |f: &dyn Fn(&Row) -> bool| rows.iter().filter(|r| f(r)).filter_map(|r| r.amount).collect::<Vec<f64>>();
        let mean = |v: Vec<f64>| if v.is_empty() { Naive::Err("#DIV/0!") } else { Naive::Num(v.iter().sum::<f64>() / v.len() as f64) };
        let count = |f: &dyn Fn(&Row) -> bool| Naive::Num(rows.iter().filter(|r| f(r)).count() as f64);
        match self.q {
            Question::Total => vec![Naive::Num(nums(&|_| true).iter().sum())],
            Question::Average => vec![mean(nums(&|_| true))],
            Question::Max => vec![Naive::Num(nums(&|_| true).into_iter().fold(0.0, f64::max))],
            Question::Orders => vec![count(&|_| true)],
            Question::SumBy => vec![Naive::Num(nums(&|r| r.sup).iter().sum())],
            Question::CountBy => vec![count(&|r| r.sup)],
            Question::CountAbove => vec![count(&|r| r.amount.is_some_and(|a| a > self.threshold))],
            Question::SumBy2 => vec![Naive::Num(nums(&|r| r.sup && r.status).iter().sum())],
            Question::CountBy2 => vec![count(&|r| r.sup && r.status)],
            Question::AverageBy => vec![mean(nums(&|r| r.sup))],
            Question::LineTotal | Question::CatalogTotal => {
                let masked: f64 = rows.iter().filter_map(|r| r.cost.ok()).sum();
                // A lookup filled down through a blank row looks up an empty code: #Н/Д.
                let blank_lookup = self.q == Question::CatalogTotal && left(Dirt::BlankRow) && self.lines.contains(&Line::Blank);
                let mut out = if blank_lookup { vec![Naive::Err("#N/A")] } else { Vec::new() };
                out.extend(match rows.iter().find_map(|r| r.cost.err()) {
                    // Summing a column with an error shows the first error; wrapping each row
                    // in ЕСЛИОШИБКА(…;0) silently drops those rows instead.
                    Some(e) => vec![Naive::Err(e), Naive::Num(masked)],
                    None => vec![Naive::Num(masked)],
                });
                out
            }
        }
    }
}

fn pick_order(r: &mut Rng, reg: &Register, taken: &[usize], want: &dyn Fn(&Order) -> bool) -> Option<usize> {
    let pool: Vec<usize> = (0..reg.orders.len()).filter(|i| !taken.contains(i) && reg.orders.get(*i).is_some_and(want)).collect();
    pool.get(r.below(pool.len().max(1))).copied()
}

fn position(lines: &[Line], line: Line) -> usize {
    lines.iter().position(|l| *l == line).unwrap_or(lines.len())
}

fn build_register(spec: &Spec, q: Question, pool: &[Dirt]) -> Register {
    let tier = spec.level.clamp(1, TIERS);
    let mut l = Rng::new(spec.layout ^ 0x11FE_0D47_A001);
    let mut d = Rng::new(spec.data ^ 0x0DA7_A5EE_D002);

    // Which dirt: tiers 1–2 take the pool in teaching order (a visible error, then the next
    // one); from tier 3 the remaining kinds join in random order.
    let k = usize::from(tier).min(pool.len());
    let mut present: Vec<Dirt> = pool.iter().copied().take(k.min(2)).collect();
    let mut rest: Vec<Dirt> = pool.iter().copied().skip(2).collect();
    l.shuffle(&mut rest);
    present.extend(rest.into_iter().take(k.saturating_sub(2)));

    let mut names: Vec<&'static str> = SUPPLIERS.to_vec();
    l.shuffle(&mut names);
    let mut suppliers: Vec<&'static str> = names.into_iter().take(3).collect();
    suppliers.push(SIDE_SUPPLIER);

    let n = if tier <= 2 { 10 + d.below(4) } else { 13 + d.below(4) };
    let start_no = 100 + d.int(0, 700);
    let mut day = date(2026, 3, 2);
    let mut orders = Vec::new();
    for i in 0..n {
        let item = d.below(ITEMS.len());
        let base = ITEMS.get(item).map(|x| x.1).unwrap_or(1_000);
        let price = ((base as f64 * (0.9 + d.below(21) as f64 / 100.0)) / 10.0).round() * 10.0;
        let qty = if base < 1_000 {
            d.int(20, 300)
        } else if base > 20_000 {
            d.int(1, 8)
        } else {
            d.int(2, 40)
        } as f64;
        // Every question needs enough rows of its own supplier/status to be a real question.
        let sup = if i % 3 == 1 || i % 7 == 5 { 0 } else { 1 + d.below(suppliers.len() - 1) };
        let status = if sup == 0 && i % 4 != 3 { 0 } else { d.below(STATUSES.len()) };
        orders.push(Order {
            no: format!("PO-{}", start_no + i as i64),
            date: day,
            sup,
            item,
            qty,
            price,
            status,
            comment: d.pick(COMMENTS),
            dirt: Vec::new(),
        });
        day += d.int(0, 2) as f64;
    }
    // Guarantee at least one relevant non-status row for status questions.
    if let Some(o) = orders.iter_mut().filter(|o| o.sup == 0).nth(1) {
        o.status = 1;
    }

    // Threshold strictly between amounts, so "дороже" never meets an equal amount.
    let mut amounts: Vec<f64> = orders.iter().map(Order::amount).collect();
    amounts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = amounts.get(amounts.len() / 2).copied().unwrap_or(50_000.0);
    let mut threshold = ((mid / 10_000.0).round() * 10_000.0).max(10_000.0);
    while amounts.iter().any(|a| (a - threshold).abs() < 0.5) {
        threshold += 5_000.0;
    }

    let catalog: Vec<f64> = ITEMS.iter().map(|(_, p)| *p as f64).collect();
    let mut reg = Register { q, orders, lines: Vec::new(), suppliers, threshold, catalog, present: present.clone() };
    if q == Question::CatalogTotal {
        for o in &mut reg.orders {
            o.price = reg.catalog.get(o.item).copied().unwrap_or(o.price);
        }
    }

    // MAX: the largest order sits in the second half, so a blank row can hide it.
    if q == Question::Max {
        let m = (0..n).max_by(|a, b| {
            let x = reg.orders.get(*a).map(Order::amount).unwrap_or(0.0);
            let y = reg.orders.get(*b).map(Order::amount).unwrap_or(0.0);
            x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
        });
        let target = n / 2 + d.below(n - n / 2);
        if let Some(m) = m
            && m != target
            && m < reg.orders.len()
            && target < reg.orders.len()
        {
            let (a, b) = (m.min(target), m.max(target));
            let (left, right) = reg.orders.split_at_mut(b);
            if let (Some(x), Some(y)) = (left.get_mut(a), right.first_mut()) {
                std::mem::swap(&mut x.item, &mut y.item);
                std::mem::swap(&mut x.qty, &mut y.qty);
                std::mem::swap(&mut x.price, &mut y.price);
            }
        }
    }
    let argmax = (0..n)
        .max_by(|a, b| {
            let x = reg.orders.get(*a).map(Order::amount).unwrap_or(0.0);
            let y = reg.orders.get(*b).map(Order::amount).unwrap_or(0.0);
            x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0);

    // Cell-level dirt on relevant rows (the max row for MAX).
    let mut taken: Vec<usize> = Vec::new();
    let several = tier >= 3;
    for kind in present.clone() {
        let rows = match kind {
            Dirt::NameSpaces if several => 2,
            Dirt::NameSpaces | Dirt::NameVariant | Dirt::StatusVariant | Dirt::AmountUnit | Dirt::PriceUnit | Dirt::ArticleNoDash => 1,
            Dirt::AmountText | Dirt::QtyUnit | Dirt::ArticleSpace => 1 + usize::from(several),
            Dirt::Subtotal | Dirt::BlankRow | Dirt::Duplicate => 0,
        };
        for _ in 0..rows {
            let i = if q == Question::Max && matches!(kind, Dirt::AmountText | Dirt::AmountUnit) && !taken.contains(&argmax) {
                Some(argmax)
            } else {
                let relevant = |o: &Order| match kind {
                    Dirt::NameSpaces | Dirt::NameVariant => o.sup == 0 && (!matches!(q, Question::SumBy2 | Question::CountBy2) || o.status == 0),
                    Dirt::StatusVariant => o.sup == 0 && o.status == 0,
                    _ => reg.relevant(o),
                };
                pick_order(&mut d, &reg, &taken, &relevant)
            };
            if let Some(i) = i {
                taken.push(i);
                if let Some(o) = reg.orders.get_mut(i) {
                    o.dirt.push(kind);
                }
            }
        }
    }

    reg.lines = (0..n).map(Line::Order).collect();
    if present.contains(&Dirt::Subtotal) {
        // For MAX the subtotal follows the largest order, so it is bigger than any order.
        let s = if q == Question::Max { (argmax + 1).max(3) } else { (n / 2).max(3) };
        let v: f64 = reg.orders.iter().take(s).map(Order::amount).sum();
        reg.lines.insert(s, Line::Subtotal(v));
    }
    if present.contains(&Dirt::Duplicate)
        && let Some(j) = pick_order(&mut d, &reg, &taken, &|o: &Order| reg.relevant(o) && o.dirt.is_empty())
    {
        taken.push(j);
        let from = position(&reg.lines, Line::Order(j)) + 2;
        let at = from + d.below(reg.lines.len().saturating_sub(from).max(1));
        reg.lines.insert(at.min(reg.lines.len()), Line::Dup(j));
    }
    if present.contains(&Dirt::BlankRow) {
        // Before the last relevant row, so the rows the blank cuts off matter.
        let last = reg
            .lines
            .iter()
            .rposition(
                |line| matches!(line, Line::Order(i) if reg.orders.get(*i).is_some_and(|o| reg.relevant(o)) && (q != Question::Max || *i == argmax)),
            )
            .unwrap_or(reg.lines.len());
        let lo = (last / 2).max(2);
        let at = lo + d.below(last.saturating_sub(lo).max(1));
        reg.lines.insert(at.clamp(1, last.max(1)), Line::Blank);
    }
    reg
}

impl Register {
    fn has(&self, kind: Dirt) -> bool {
        match kind {
            Dirt::Subtotal => self.lines.iter().any(|l| matches!(l, Line::Subtotal(_))),
            Dirt::BlankRow => self.lines.contains(&Line::Blank),
            Dirt::Duplicate => self.lines.iter().any(|l| matches!(l, Line::Dup(_))),
            _ => self.orders.iter().any(|o| o.dirt.contains(&kind)),
        }
    }

    /// Dirt that is really in the table and really changes the answer.
    fn effective(&self) -> Vec<Dirt> {
        let right = self.measure(0);
        let same = |a: &Naive, b: &Naive| match (a, b) {
            (Naive::Num(x), Naive::Num(y)) => (x - y).abs() < 0.005,
            _ => a == b,
        };
        self.present
            .iter()
            .copied()
            .filter(|k| self.has(*k))
            .filter(|k| {
                let wrong = self.measure(k.bit());
                wrong.len() != right.len() || wrong.iter().zip(&right).any(|(a, b)| !same(a, b))
            })
            .collect()
    }
}

/// The register for a spec. Data is redrawn (deterministically) until every chosen kind of
/// dirt is a real trap, so a tier never promises errors that do not matter.
fn register(spec: &Spec, q: Question, pool: &[Dirt]) -> Register {
    let mut best: Option<Register> = None;
    for attempt in 0..12u64 {
        let mut s = spec.clone();
        s.data ^= attempt.wrapping_mul(0x9E37_79B9_7F4A_7C00) & !0xFF;
        let reg = build_register(&s, q, pool);
        if reg.effective().len() == reg.present.len() {
            return reg;
        }
        if best.as_ref().is_none_or(|b| b.effective().len() < reg.effective().len()) {
            best = Some(reg);
        }
    }
    best.unwrap_or_else(|| build_register(spec, q, pool))
}

fn variant_name(name: &str, pick: usize) -> String {
    let core = name.strip_prefix("ТОО ").unwrap_or(name);
    match pick % 3 {
        0 => format!("{core} ТОО"),
        1 => format!("ТОО «{core}»"),
        _ => core.to_string(),
    }
}

fn spaced_name(name: &str, pick: usize) -> String {
    match pick % 3 {
        0 => format!("{name} "),
        1 => format!(" {name}"),
        _ => name.replacen(' ', "  ", 1),
    }
}

/// Column layout of the register, relative to its left edge.
struct Cols {
    no: u32,
    date: u32,
    sup: u32,
    item: u32,
    qty: u32,
    price: u32,
    amount: u32,
    status: u32,
    comment: u32,
    right: u32,
}

const REGISTER: Cols = Cols { no: 0, date: 1, sup: 2, item: 3, qty: 4, price: 5, amount: 6, status: 7, comment: 8, right: 8 };
/// Lookup layout: orders (№, артикул, кол-во, комментарий), then the catalog at column F.
const ORDERS: Cols = Cols { no: 0, date: 0, sup: 0, item: 1, qty: 2, price: 0, amount: 0, status: 0, comment: 3, right: 3 };
const CATALOG_COL: u32 = 5;

/// The per-row step of a catalog total (tiers 1–2): its own column between orders and catalog.
const HELPER_COL: u32 = 4;

/// Catalog price × quantity for sheet row `r`; the catalog is pinned with $ for filling down.
fn helper_formula(r: u32, first: u32) -> String {
    let (c, l) = (crate::sheet::col_name(CATALOG_COL), crate::sheet::col_name(CATALOG_COL + 2));
    format!("=VLOOKUP(B{r},${c}${first}:${l}${},3,FALSE)*C{r}", first + ITEMS.len() as u32 - 1)
}

fn article(item: usize) -> String {
    format!("OM-{}", 101 + item)
}

pub fn build(spec: &Spec) -> Result<Task> {
    let skill = base(&spec.skill).ok_or_else(|| crate::LearnError::Invalid(format!("Нет живой задачи {}", spec.skill)))?;
    let (q, pool) = question(skill).ok_or_else(|| crate::LearnError::Invalid(format!("Нет живой задачи {}", spec.skill)))?;
    let tier = spec.level.clamp(1, TIERS);
    let reg = register(spec, q, pool);
    let mut l = Rng::new(spec.layout ^ 0x5EED_0F11_0003);
    let top = top_of(spec);
    let header = top + 1;
    let first = header + 1;
    let row_of = |line: usize| first + line as u32;
    let lookup = q == Question::CatalogTotal;
    let c = if lookup { &ORDERS } else { &REGISTER };
    let mut setup = Setup::default();

    let title = if lookup {
        "Заказы на март (вели вручную) и каталог поставщика"
    } else {
        "Реестр закупок, март 2026 (вели вручную)"
    };
    setup.text(a1(top, 0), title);
    setup.bold.push(a1(top, 0));
    let headers: Vec<&str> = if lookup {
        vec!["№ заказа", "Артикул", "Кол-во", "Комментарий"]
    } else {
        vec!["№ заказа", "Дата", "Поставщик", "Товар", "Кол-во", "Цена, тг", "Сумма, тг", "Статус", "Комментарий"]
    };
    setup.header(header, 0, &headers);
    let widths: &[(u32, f32)] = if lookup {
        &[(0, 110.0), (1, 110.0), (2, 80.0), (3, 190.0), (4, 30.0), (5, 100.0), (6, 200.0), (7, 100.0)]
    } else {
        &[(0, 120.0), (1, 95.0), (2, 175.0), (3, 195.0), (4, 75.0), (5, 95.0), (6, 105.0), (7, 90.0), (8, 175.0), (9, 30.0)]
    };
    for (col, w) in widths {
        setup.width(*col, *w);
    }

    // Rendering: what each order looks like after hand entry.
    let mut where_dirt: Vec<(Dirt, u32, String)> = Vec::new();
    let mut pick = l.below(3);
    let lower_noise = tier >= 2;
    let mut noise_done = false;
    for (li, line) in reg.lines.iter().enumerate() {
        let r = row_of(li);
        match *line {
            Line::Blank => {}
            Line::Subtotal(v) => {
                let label = "Итого 1-я половина";
                setup.input(a1(r, c.no), Cell::Text(label.into()));
                setup.input(a1(r, c.amount), Cell::Num(v));
                where_dirt.push((Dirt::Subtotal, r, label.into()));
            }
            Line::Order(i) | Line::Dup(i) => {
                let Some(o) = reg.orders.get(i) else { continue };
                let dup = matches!(line, Line::Dup(_));
                let dirty = |d: Dirt| !dup && o.dirt.contains(&d);
                if dup {
                    where_dirt.push((Dirt::Duplicate, r, o.no.clone()));
                }
                setup.input(a1(r, c.no), Cell::Text(o.no.clone()));
                if lookup {
                    let mut art = article(o.item);
                    if dirty(Dirt::ArticleSpace) {
                        art.push(' ');
                        where_dirt.push((Dirt::ArticleSpace, r, art.clone()));
                    } else if dirty(Dirt::ArticleNoDash) {
                        art = art.replace('-', "");
                        where_dirt.push((Dirt::ArticleNoDash, r, art.clone()));
                    } else if lower_noise && !noise_done {
                        // Harmless: lookups ignore letter case.
                        art = art.to_lowercase();
                        noise_done = true;
                    }
                    setup.input(a1(r, c.item), Cell::Text(art));
                } else {
                    setup.input(a1(r, c.date), Cell::Date(o.date));
                    let name = reg.suppliers.get(o.sup).copied().unwrap_or(SIDE_SUPPLIER);
                    let shown = if dirty(Dirt::NameSpaces) {
                        pick += 1;
                        let s = spaced_name(name, pick);
                        where_dirt.push((Dirt::NameSpaces, r, s.clone()));
                        s
                    } else if dirty(Dirt::NameVariant) {
                        pick += 1;
                        let s = variant_name(name, pick);
                        where_dirt.push((Dirt::NameVariant, r, s.clone()));
                        s
                    } else if lower_noise && !noise_done && o.sup == 0 {
                        // Harmless: СУММЕСЛИ and СЧЁТЕСЛИ ignore letter case.
                        noise_done = true;
                        name.to_lowercase()
                    } else {
                        name.to_string()
                    };
                    setup.input(a1(r, c.sup), Cell::Text(shown));
                    setup.input(a1(r, c.item), Cell::Text(ITEMS.get(o.item).map(|x| x.0).unwrap_or("Товар").into()));
                    let status = STATUSES.get(o.status).copied().unwrap_or("Заказан");
                    let shown = if dirty(Dirt::StatusVariant) {
                        pick += 1;
                        // Tier 1 shows a spelling you can see; the trailing space (invisible) only
                        // joins from tier 2, after the visible kind has been met.
                        let s = if tier == 1 {
                            ["Получено", "получен."][pick % 2]
                        } else {
                            ["Получено", "Получен ", "получен."][pick % 3]
                        }
                        .to_string();
                        where_dirt.push((Dirt::StatusVariant, r, s.clone()));
                        s
                    } else {
                        status.to_string()
                    };
                    setup.input(a1(r, c.status), Cell::Text(shown));
                    if dirty(Dirt::PriceUnit) {
                        let s = format!("{} тг", o.price as i64);
                        where_dirt.push((Dirt::PriceUnit, r, s.clone()));
                        setup.input(a1(r, c.price), Cell::Text(s));
                    } else {
                        setup.input(a1(r, c.price), Cell::Num(o.price));
                    }
                    if q != Question::LineTotal {
                        let amount = o.amount();
                        if dirty(Dirt::AmountText) {
                            // An apostrophe makes the engine keep the digits as text, as in a 1С export.
                            setup.input(a1(r, c.amount), Cell::Text(format!("'{}", amount as i64)));
                            where_dirt.push((Dirt::AmountText, r, format!("{}", amount as i64)));
                        } else if dirty(Dirt::AmountUnit) {
                            let s = format!("{} тг", amount as i64);
                            where_dirt.push((Dirt::AmountUnit, r, s.clone()));
                            setup.input(a1(r, c.amount), Cell::Text(s));
                        } else {
                            setup.input(a1(r, c.amount), Cell::Num(amount));
                        }
                    }
                }
                if dirty(Dirt::QtyUnit) {
                    let s = format!("{} шт", o.qty as i64);
                    where_dirt.push((Dirt::QtyUnit, r, s.clone()));
                    setup.input(a1(r, c.qty), Cell::Text(s));
                } else {
                    setup.input(a1(r, c.qty), Cell::Num(o.qty));
                }
                if !o.comment.is_empty() {
                    setup.input(a1(r, c.comment), Cell::Text(o.comment.into()));
                }
            }
        }
    }
    where_dirt.extend(blank_rows(&reg, first));
    let last = row_of(reg.lines.len().saturating_sub(1));
    if lookup {
        setup.header(header, CATALOG_COL, &["Артикул", "Товар", "Цена, тг"]);
        for (i, (name, price)) in ITEMS.iter().enumerate() {
            let r = first + i as u32;
            setup.input(a1(r, CATALOG_COL), Cell::Text(article(i)));
            setup.input(a1(r, CATALOG_COL + 1), Cell::Text((*name).into()));
            setup.input(a1(r, CATALOG_COL + 2), Cell::Num(*price as f64));
        }
        setup.formats.push((format!("{}:{}", a1(first, CATALOG_COL + 2), a1(first + ITEMS.len() as u32, CATALOG_COL + 2)), "#,##0".into()));
        // Tiers 1–2 lay out the per-row step: a «Сумма строки» column between orders and catalog.
        // Tier 1 fills it, so the #Н/Д of a broken article points at its row by itself.
        if tier <= 2 {
            setup.text(a1(header, HELPER_COL), "Сумма строки");
            setup.bold.push(a1(header, HELPER_COL));
            setup.width(HELPER_COL, 120.0);
            if tier == 1 {
                for (li, line) in reg.lines.iter().enumerate() {
                    if matches!(line, Line::Order(_) | Line::Dup(_)) {
                        let r = row_of(li);
                        setup.put(a1(r, HELPER_COL), Cell::Formula(helper_formula(r, first)), false);
                    }
                }
            }
            setup.formats.push((format!("{}:{}", a1(first, HELPER_COL), a1(last, HELPER_COL)), "#,##0".into()));
        }
    } else {
        setup.formats.push((format!("{}:{}", a1(first, c.price), a1(last, c.amount)), "#,##0".into()));
    }

    let expected = match reg.measure(0).first() {
        Some(Naive::Num(v)) => *v,
        _ => return Err(crate::LearnError::Invalid("Живая задача без ответа".into())),
    };

    // Only dirt that is really there and really changes the answer is a trap worth naming.
    let kinds = reg.effective();
    // Wrong answers for every subset of untreated dirt, smallest subsets first.
    let mut subsets: Vec<u32> = (1..(1u32 << kinds.len())).collect();
    subsets.sort_by_key(|m| m.count_ones());
    let mut misses: Vec<(Expectation, String)> = Vec::new();
    for m in subsets {
        let chosen: Vec<Dirt> = kinds.iter().enumerate().filter(|(i, _)| m & (1 << i) != 0).map(|(_, k)| *k).collect();
        let mask = chosen.iter().fold(0u16, |acc, k| acc | k.bit());
        let why = chosen.iter().map(|k| explain(*k, &where_dirt, q)).collect::<Vec<_>>().join("; ");
        for naive in reg.measure(mask) {
            let e = match naive {
                Naive::Num(v) if (v - expected).abs() < 0.005 => continue,
                Naive::Num(v) => Expectation::Number(v),
                Naive::Err(e) => Expectation::Error(e.into()),
            };
            if !misses.iter().any(|(x, _)| *x == e) {
                misses.push((e, why.clone()));
            }
        }
    }

    // The answer cell fades: highlighted (tier 1), labelled (tier 2), self-placed (3–4).
    let target = reg.suppliers.first().copied().unwrap_or("ТОО Вектор");
    let label = match q {
        Question::Total => "Всего закупок:".to_string(),
        Question::Average => "Средний заказ:".to_string(),
        Question::Max => "Крупнейший заказ:".to_string(),
        Question::Orders => "Заказов в реестре:".to_string(),
        Question::SumBy => format!("Закупки у {target}:"),
        Question::CountBy => format!("Заказов у {target}:"),
        Question::CountAbove => format!("Заказов дороже {}:", num_text(reg.threshold)),
        Question::SumBy2 => format!("Получено от {target}, тг:"),
        Question::CountBy2 => format!("Получено заказов {target}:"),
        Question::AverageBy => format!("Средний заказ {target}:"),
        Question::LineTotal | Question::CatalogTotal => "Итого по заказу:".to_string(),
    };
    // Right of everything given (the catalog sits right of the orders in lookup tasks).
    let answer_col = if lookup { CATALOG_COL + 4 } else { c.right + 2 };
    let cell = (tier <= 2).then(|| a1(header, answer_col + 1));
    if let Some(cell) = &cell {
        setup.text(a1(header, answer_col), label.as_str());
        setup.bold.push(a1(header, answer_col));
        setup.width(answer_col, 230.0);
        setup.width(answer_col + 1, 120.0);
        if tier == 1 {
            setup.answer(cell.clone());
        }
    } else {
        setup.width(answer_col, 120.0);
        setup.width(answer_col + 1, 120.0);
    }

    let ask = match q {
        Question::Total => "Сколько всего мы потратили на закупки по этому реестру?".to_string(),
        Question::Average => "Какая у нас средняя сумма одного заказа?".to_string(),
        Question::Max => "Какой у нас самый крупный заказ — на какую сумму?".to_string(),
        Question::Orders => "Сколько всего заказов в реестре?".to_string(),
        Question::SumBy => format!("Сколько мы закупили у {target}?"),
        Question::CountBy => format!("Сколько заказов мы сделали у {target}?"),
        Question::CountAbove => format!("Сколько заказов дороже {} тг? Их согласует директор.", num_text(reg.threshold)),
        Question::SumBy2 => format!("На какую сумму мы уже получили товар от {target} (статус «Получен»)?"),
        Question::CountBy2 => format!("Сколько заказов {target} уже получено (статус «Получен»)?"),
        Question::AverageBy => format!("Какой средний заказ у {target}?"),
        Question::LineTotal => "Посчитай сумму каждой строки (Кол-во × Цена) и общий итог заказа.".to_string(),
        Question::CatalogTotal => "Подтяни цены из каталога и посчитай, на сколько у нас заказов всего.".to_string(),
    };
    let rule = match (lookup, tier) {
        (true, 1) => format!(
            " В столбце {h} уже посчитана сумма каждой строки: цена из каталога × количество. Где вместо суммы #Н/Д, артикул не нашёлся в каталоге: сравни его с каталогом и исправь.",
            h = crate::sheet::col_name(HELPER_COL)
        ),
        (true, 2) => format!(
            " Сначала посчитай в столбце {h} сумму каждой строки (цена из каталога × количество) и протяни вниз. Где получится #Н/Д, артикул не нашёлся в каталоге: сравни его с каталогом и исправь.",
            h = crate::sheet::col_name(HELPER_COL)
        ),
        (true, _) => String::new(),
        (false, _) => " Правило реестра: один номер заказа — один заказ.".to_string(),
    };
    let place = match &cell {
        Some(cell) => format!(" Ответ — формулой в {cell}."),
        None => format!(" Ответ оформи сам: справа или ниже таблицы напиши «{ANSWER_LABEL}», а рядом с этой подписью — формулу."),
    };
    let warn = if tier <= 2 { " Таблицу вели вручную — проверь данные." } else { "" };
    let prompt = format!("Андрей, снабжение: «{ask}»{warn}{rule}{place}");

    let mut cols: Vec<&str> = Vec::new();
    let mut tools: Vec<&str> = Vec::new();
    for k in &kinds {
        if !cols.contains(&k.column()) {
            cols.push(k.column());
        }
        if !tools.contains(&k.tool()) {
            tools.push(k.tool());
        }
    }
    let concrete = format!("Конкретно: {}.", kinds.iter().map(|k| explain(*k, &where_dirt, q)).collect::<Vec<_>>().join("; "));
    let hints = if lookup && tier <= 2 {
        // The error shows itself: #Н/Д in the row column marks the article to compare.
        let h = crate::sheet::col_name(HELPER_COL);
        let total = format!("=СУММ({h}{first}:{h}{last})");
        let row_step = crate::sheet::ru(&helper_formula(first, first));
        vec![
            format!(
                "#Н/Д в столбце {h} показывает строку с ошибкой: ВПР не нашёл этот артикул в каталоге. Сравни его с каталогом символ в символ.{}",
                if tier == 1 {
                    ""
                } else {
                    " Если на вид он такой же, в нём лишний пробел: глазом его не видно, просто набери артикул заново."
                }
            ),
            if tier == 1 {
                format!("Исправь артикул прямо в ячейке: сумма строки пересчитается сама. Итог по заказу: {total}")
            } else {
                format!("Сумма первой строки: {row_step}, затем протяни вниз ($ держит каталог на месте). Итог по заказу: {total}")
            },
            concrete,
        ]
    } else {
        vec![
            format!("Прежде чем считать, пробеги глазами столбцы {} — там есть ошибки ввода.", cols.join(", ")),
            format!(
                "Как справиться: {}. Проверь себя формулой: {}.",
                tools.join("; "),
                kinds.iter().map(|k| control(*k, c, first, last, target)).collect::<Vec<_>>().join("; ")
            ),
            concrete,
        ]
    };

    // Cells with an entry error: a formula that takes its condition from one repeats the error.
    let dirty: Vec<(String, String)> = where_dirt
        .iter()
        .filter_map(|(kind, r, shown)| {
            let col = match kind {
                Dirt::NameSpaces | Dirt::NameVariant => c.sup,
                Dirt::StatusVariant => c.status,
                Dirt::AmountText | Dirt::AmountUnit => c.amount,
                Dirt::QtyUnit => c.qty,
                Dirt::PriceUnit => c.price,
                Dirt::ArticleSpace | Dirt::ArticleNoDash => c.item,
                Dirt::Subtotal | Dirt::BlankRow | Dirt::Duplicate => return None,
            };
            let at = a1(*r, col);
            let why = match kind {
                Dirt::NameSpaces | Dirt::NameVariant => {
                    format!("Условие взято из {at}, а там ошибка ввода: «{shown}». Напиши «{target}» сам или исправь {at}")
                }
                Dirt::StatusVariant => format!("Условие взято из {at}, а там ошибка ввода: «{shown}». Напиши «Получен» сам или исправь {at}"),
                _ => format!("Формула берёт {at}, а там ошибка ввода: «{shown}». Сначала исправь эту ячейку"),
            };
            Some((at, why))
        })
        .collect();
    let control = kinds.iter().map(|k| control(*k, c, first, last, target)).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("; ");
    let focus = cell.clone().unwrap_or_else(|| a1(header, answer_col));
    let check = Check {
        label: label.trim_end_matches(':').to_string(),
        skill: skill.to_string(),
        kind: Kind::Live { cell, top, expected, misses, dirty, control },
    };
    Ok(Task {
        spec: spec.clone(),
        mode: Mode::Drill,
        title: "Живые данные".into(),
        prompt,
        example: None,
        hints,
        focus,
        setup,
        checks: vec![check],
        goal_seconds: 60.0 + 30.0 * f32::from(tier),
        // Cleaning the source by hand is legitimate work here; the oracle is independent.
        reorders: true,
        brief: None,
        sql: None,
    })
}

/// A formula that proves this kind of error is gone: it gives 0 (or a match) on a clean table.
fn control(kind: Dirt, c: &Cols, first: u32, last: u32, target: &str) -> String {
    let col = |i: u32| crate::sheet::col_name(i);
    let range = |i: u32| format!("{0}{first}:{0}{last}", col(i));
    let amt = range(c.amount);
    match kind {
        Dirt::StatusVariant => {
            let st = range(c.status);
            format!(
                "=СУММ({amt})-СУММЕСЛИ({st};\"Получен\";{amt})-СУММЕСЛИ({st};\"В пути\";{amt})-СУММЕСЛИ({st};\"Заказан\";{amt}) должно дать 0, иначе какой-то статус записан иначе"
            )
        }
        Dirt::NameVariant | Dirt::NameSpaces => format!(
            "=СЧЁТЕСЛИ({};\"{target}\") покажет, сколько строк «{target}» видит формула; сравни с тем, сколько видишь глазами. Фильтр по столбцу «Поставщик» покажет все варианты написания",
            range(c.sup)
        ),
        Dirt::AmountText | Dirt::AmountUnit => format!("=СЧЁТЗ({amt})-СЧЁТ({amt}) должно дать 0, иначе в суммах есть число, записанное текстом"),
        Dirt::QtyUnit => format!("=СЧЁТЗ({0})-СЧЁТ({0}) должно дать 0, иначе в количестве есть текст", range(c.qty)),
        Dirt::PriceUnit => format!("=СЧЁТЗ({0})-СЧЁТ({0}) должно дать 0, иначе в ценах есть текст", range(c.price)),
        Dirt::Subtotal => {
            format!("=СЧЁТЗ({0})-СЧЁТЕСЛИ({0};\"PO-*\") должно дать 0, иначе в «№ заказа» есть строка, которая не заказ", range(c.no))
        }
        Dirt::Duplicate => format!(
            "рядом с каждой строкой =СЧЁТЕСЛИ(${0}${first}:${0}${last};{0}{first}) и протяни вниз: где 2, номер заказа повторяется",
            col(c.no)
        ),
        Dirt::BlankRow => {
            format!("=СЧЁТЗ({}) покажет, сколько заказов на самом деле; сравни с тем, сколько строк попало в твою формулу", range(c.no))
        }
        Dirt::ArticleSpace | Dirt::ArticleNoDash => format!(
            "рядом с каждой строкой =СЧЁТЕСЛИ(${0}${1}:${0}${2};{3}{first}) и протяни вниз: где 0, такого артикула нет в каталоге",
            col(CATALOG_COL),
            first,
            first + ITEMS.len() as u32 - 1,
            col(c.item)
        ),
    }
}

/// One trap, in work terms, with the rows it sits in.
fn explain(kind: Dirt, at: &[(Dirt, u32, String)], q: Question) -> String {
    let spots: Vec<&(Dirt, u32, String)> = at.iter().filter(|(k, ..)| *k == kind).collect();
    let rows = spots.iter().map(|(_, r, _)| r.to_string()).collect::<Vec<_>>().join(", ");
    let shown = spots.iter().map(|(_, r, s)| format!("строка {r}: «{s}»")).collect::<Vec<_>>().join(", ");
    let fname = match q {
        Question::SumBy | Question::SumBy2 => "СУММЕСЛИ",
        Question::AverageBy => "СРЗНАЧЕСЛИ",
        _ => "СЧЁТЕСЛИ",
    };
    match kind {
        Dirt::NameSpaces => format!(
            "лишние пробелы в названии поставщика ({shown}): глазом почти не видно, но для {fname} это другой поставщик, набери название заново"
        ),
        Dirt::NameVariant => format!("поставщик записан иначе ({shown}) — для {fname} это другое название"),
        Dirt::StatusVariant => format!("статус записан иначе ({shown}) — условие «Получен» его не видит"),
        Dirt::AmountText => format!("сумма записана текстом (строка {rows}) — СУММ и похожие функции её пропускают"),
        Dirt::AmountUnit => format!("в сумме приписано «тг» ({shown}) — это уже текст, а не число"),
        Dirt::QtyUnit => format!("в количестве приписано «шт» ({shown}) — умножение на такой текст даёт #ЗНАЧ!"),
        Dirt::PriceUnit => format!("в цене приписано «тг» ({shown}) — умножение даёт #ЗНАЧ!"),
        Dirt::Subtotal => format!("строка {rows} — промежуточный итог, а не заказ: её нельзя включать"),
        Dirt::BlankRow => {
            let blank = at.iter().find(|(k, ..)| *k == Dirt::BlankRow).map(|(_, r, _)| *r);
            match blank {
                Some(r) if q == Question::CatalogTotal => {
                    format!("пустая строка {r} разрывает таблицу: выделение обрывается на ней, а ВПР, протянутый через неё, даёт #Н/Д")
                }
                Some(r) => format!("пустая строка {r} разрывает таблицу: выделение и автосумма обрываются на ней, строки ниже не попали в расчёт"),
                None => "пустая строка разрывает таблицу: строки ниже неё не попали в расчёт".into(),
            }
        }
        Dirt::Duplicate => {
            let no = spots.first().map(|(_, _, s)| s.clone()).unwrap_or_default();
            format!("заказ {no} записан дважды (повтор в строке {rows}) — учитывай его один раз")
        }
        Dirt::ArticleSpace => format!("в артикуле лишний пробел ({shown}): глазом не видно, но ВПР такой артикул не находит, набери его заново"),
        Dirt::ArticleNoDash => format!("артикул записан без дефиса ({shown}) — в каталоге такого нет"),
    }
}

/// Blank rows carry no cell of their own; this records where they are for explanations.
fn blank_rows(reg: &Register, first: u32) -> Vec<(Dirt, u32, String)> {
    reg.lines.iter().enumerate().filter(|(_, l)| **l == Line::Blank).map(|(i, _)| (Dirt::BlankRow, first + i as u32, String::new())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::evaluate;
    use crate::sheet::{self, Cell};
    use gridcraft_engine::core::Value;

    const SKILLS: &[&str] = &[
        "fn.sum",
        "fn.average",
        "fn.minmax",
        "fn.count",
        "cond.sumif",
        "cond.countif",
        "cond.compare",
        "cond.sumifs",
        "cond.countifs",
        "cond.averageif",
        "start.mul",
        "lookup.vlookup",
    ];

    fn live_kind(task: &Task) -> (Option<String>, u32, f64, Vec<(Expectation, String)>) {
        match task.checks.first().map(|c| c.kind.clone()) {
            Some(Kind::Live { cell, top, expected, misses, .. }) => (cell, top, expected, misses),
            other => panic!("not a live check: {other:?}"),
        }
    }

    struct Ctx {
        reg: Register,
        first: u32,
        last: u32,
        target: &'static str,
    }

    fn ctx(spec: &Spec) -> Ctx {
        let (q, pool) = question(base(&spec.skill).expect("live id")).expect("question");
        let reg = register(spec, q, pool);
        let first = top_of(spec) + 2;
        let last = first + reg.lines.len() as u32 - 1;
        let target = reg.suppliers.first().copied().expect("supplier");
        Ctx { reg, first, last, target }
    }

    /// What a learner who ignores the dirt types: one formula over the whole table.
    fn naive_formula(c: &Ctx) -> Option<String> {
        let (f, l) = (c.first, c.last);
        let t = c.target;
        Some(match c.reg.q {
            Question::Total => format!("=SUM(G{f}:G{l})"),
            Question::Average => format!("=AVERAGE(G{f}:G{l})"),
            Question::Max => format!("=MAX(G{f}:G{l})"),
            Question::Orders => format!("=COUNTA(A{f}:A{l})"),
            Question::SumBy => format!("=SUMIF(C{f}:C{l},\"{t}\",G{f}:G{l})"),
            Question::CountBy => format!("=COUNTIF(C{f}:C{l},\"{t}\")"),
            Question::CountAbove => format!("=COUNTIF(G{f}:G{l},\">{}\")", c.reg.threshold),
            Question::SumBy2 => format!("=SUMIFS(G{f}:G{l},C{f}:C{l},\"{t}\",H{f}:H{l},\"Получен\")"),
            Question::CountBy2 => format!("=COUNTIFS(C{f}:C{l},\"{t}\",H{f}:H{l},\"Получен\")"),
            Question::AverageBy => format!("=AVERAGEIF(C{f}:C{l},\"{t}\",G{f}:G{l})"),
            Question::LineTotal | Question::CatalogTotal => return None,
        })
    }

    /// Per-row helper column K, as a learner fills it down, then the total of K.
    fn fill_helpers(s: &mut gridcraft_engine::Session, c: &Ctx, skip_blank: bool) -> String {
        let cat = format!("$F${}:$H${}", c.first, c.first + ITEMS.len() as u32 - 1);
        for (i, line) in c.reg.lines.iter().enumerate() {
            if skip_blank && *line == Line::Blank {
                continue;
            }
            let r = c.first + i as u32;
            let f = if c.reg.q == Question::CatalogTotal { format!("=VLOOKUP(B{r},{cat},3,FALSE)*C{r}") } else { format!("=E{r}*F{r}") };
            sheet::set(s, &format!("K{r}"), &Cell::Formula(f)).expect("helper");
        }
        format!("=SUM(K{}:K{})", c.first, c.last)
    }

    fn engine_value(task: &Task, formula: &str) -> Value {
        let mut s = task.session().expect("session");
        sheet::set(&mut s, "ZZ1", &Cell::Formula(formula.into())).expect("formula");
        sheet::value(&s, "ZZ1").expect("value")
    }

    fn same(v: &Value, n: &Naive) -> bool {
        match (v, n) {
            (Value::Number(a), Naive::Num(b)) => (a - b).abs() < 0.005,
            (Value::Error(e), Naive::Err(x)) => e.as_str() == *x,
            _ => false,
        }
    }

    #[test]
    fn every_tier_has_real_traps_and_the_engine_agrees_with_the_model() {
        let mut thin = 0;
        let mut total = 0;
        for skill in SKILLS {
            for tier in 1..=TIERS {
                for seed in 1..=40u64 {
                    let spec = spec(skill, tier, seed * 7919, 1);
                    let task = build(&spec).expect("build");
                    let (cell, _, expected, misses) = live_kind(&task);
                    assert!(expected.is_finite(), "{skill} t{tier} s{seed}");
                    assert!(!misses.is_empty(), "{skill} t{tier} s{seed}: dirt never changes the answer");
                    assert_eq!(cell.is_some(), tier <= 2);
                    assert_eq!(task.hints.len(), 3);
                    let c = ctx(&spec);
                    total += 1;
                    if c.reg.effective().len() < usize::from(tier).min(c.reg.present.len()) {
                        thin += 1;
                    }
                    // The model of "dirt left in place" must be what the engine really computes.
                    let mask = c.reg.present.iter().filter(|k| **k != Dirt::BlankRow).fold(0u16, |m, k| m | k.bit());
                    let model = c.reg.measure(mask);
                    let got = match naive_formula(&c) {
                        Some(f) => engine_value(&task, &f),
                        None => {
                            let mut s = task.session().expect("session");
                            let f = fill_helpers(&mut s, &c, true);
                            sheet::set(&mut s, "ZZ1", &Cell::Formula(f)).expect("sum");
                            sheet::value(&s, "ZZ1").expect("value")
                        }
                    };
                    assert!(model.iter().any(|n| same(&got, n)), "{skill} t{tier} s{seed}: engine {got:?}, model {model:?}");
                }
            }
        }
        assert_eq!(thin, 0, "registers with fewer real traps than the tier promises: {thin}/{total}");
    }

    /// Fixes every dirty cell by hand and deletes repeated/subtotal rows, as a careful learner does.
    fn clean_by_hand(s: &mut gridcraft_engine::Session, c: &Ctx) {
        for (i, line) in c.reg.lines.iter().enumerate() {
            let r = c.first + i as u32;
            match *line {
                Line::Subtotal(_) | Line::Dup(_) => {
                    sheet::execute_range(s, "edit.clearContents", "range", &format!("A{r}:I{r}"), serde_json::json!({})).expect("clear");
                }
                Line::Order(o) => {
                    let o = c.reg.orders.get(o).expect("order");
                    if c.reg.q == Question::CatalogTotal {
                        sheet::set(s, &format!("B{r}"), &Cell::Text(article(o.item))).expect("art");
                        sheet::set(s, &format!("C{r}"), &Cell::Num(o.qty)).expect("qty");
                    } else {
                        let name = c.reg.suppliers.get(o.sup).copied().expect("sup");
                        sheet::set(s, &format!("C{r}"), &Cell::Text(name.into())).expect("sup");
                        sheet::set(s, &format!("H{r}"), &Cell::Text(STATUSES[o.status].into())).expect("status");
                        sheet::set(s, &format!("E{r}"), &Cell::Num(o.qty)).expect("qty");
                        sheet::set(s, &format!("F{r}"), &Cell::Num(o.price)).expect("price");
                        if c.reg.q != Question::LineTotal {
                            sheet::set(s, &format!("G{r}"), &Cell::Num(o.amount())).expect("amount");
                        }
                    }
                }
                Line::Blank => {}
            }
        }
    }

    fn answer(task: &Task, s: &mut gridcraft_engine::Session, formula: &str) {
        let (cell, top, ..) = live_kind(task);
        match cell {
            Some(cell) => sheet::set(s, &cell, &Cell::Formula(formula.into())).expect("answer"),
            None => {
                let r = top + 1;
                sheet::set(s, &format!("M{r}"), &Cell::Text(format!("{ANSWER_LABEL}:"))).expect("label");
                sheet::set(s, &format!("N{r}"), &Cell::Formula(formula.into())).expect("answer");
            }
        }
    }

    #[test]
    fn cleaning_by_hand_then_a_plain_formula_passes_at_every_tier() {
        for skill in SKILLS {
            for tier in 1..=TIERS {
                for seed in [3u64, 58, 911] {
                    let spec = spec(skill, tier, seed, 1);
                    let task = build(&spec).expect("build");
                    let c = ctx(&spec);
                    let mut s = task.session().expect("session");
                    clean_by_hand(&mut s, &c);
                    let f = match naive_formula(&c) {
                        Some(f) => f,
                        None => fill_helpers(&mut s, &c, true),
                    };
                    answer(&task, &mut s, &f);
                    let e = evaluate(&task, &s, &[]).expect("evaluate");
                    assert!(e.passed(), "{skill} t{tier} s{seed}: {:?}", e.outcomes.iter().map(|o| &o.feedback).collect::<Vec<_>>());
                }
            }
        }
    }

    #[test]
    fn a_formula_that_ignores_the_dirt_fails_and_names_the_trap() {
        for skill in SKILLS {
            for seed in [5u64, 77, 404] {
                let spec = spec(skill, 1, seed, 1);
                let task = build(&spec).expect("build");
                let c = ctx(&spec);
                let mut s = task.session().expect("session");
                let f = match naive_formula(&c) {
                    Some(f) => f,
                    None => fill_helpers(&mut s, &c, false),
                };
                answer(&task, &mut s, &f);
                let e = evaluate(&task, &s, &[]).expect("evaluate");
                let fb = e.outcomes.first().map(|o| o.feedback.clone()).unwrap_or_default();
                if e.passed() {
                    // Blank rows only bite a range that stops on them; a whole-range formula is fine.
                    assert!(c.reg.present == vec![Dirt::BlankRow] || c.reg.q == Question::LineTotal, "{skill} s{seed}: dirt did not matter");
                    continue;
                }
                assert!(fb.contains("Похоже, не учтено") || fb.contains("ошибки ввода"), "{skill} s{seed}: {fb}");
                let row_named = (c.first..=c.last).any(|r| fb.contains(&r.to_string()));
                assert!(row_named || fb.contains("ошибки ввода"), "{skill} s{seed}: {fb}");
            }
        }
    }

    #[test]
    fn first_lookup_tier_points_at_its_visible_error_and_passes_once_fixed() {
        for seed in [3u64, 58, 911] {
            let spec = spec("lookup.vlookup", 1, seed, 1);
            let task = build(&spec).expect("build");
            let c = ctx(&spec);
            assert_eq!(c.reg.present, vec![Dirt::ArticleNoDash], "tier 1 starts with an error you can see");
            let mut s = task.session().expect("session");
            let h = sheet::col_name(HELPER_COL);
            for (i, line) in c.reg.lines.iter().enumerate() {
                let r = c.first + i as u32;
                let Line::Order(o) = *line else { continue };
                let dirty = c.reg.orders.get(o).is_some_and(|o| o.dirt.contains(&Dirt::ArticleNoDash));
                let v = sheet::value(&s, &format!("{h}{r}")).expect("value");
                assert_eq!(matches!(v, Value::Error(_)), dirty, "s{seed} row {r}: {v:?}");
            }
            let total = format!("=SUM({h}{}:{h}{})", c.first, c.last);
            answer(&task, &mut s, &total);
            assert!(!evaluate(&task, &s, &[]).expect("evaluate").passed(), "the broken article must block the total");
            clean_by_hand(&mut s, &c);
            let e = evaluate(&task, &s, &[]).expect("evaluate");
            assert!(e.passed(), "s{seed}: {:?}", e.outcomes.iter().map(|o| &o.feedback).collect::<Vec<_>>());
        }
    }

    #[test]
    fn a_condition_taken_from_a_broken_cell_is_named_and_unknown_misses_get_a_control_formula() {
        for seed in [3u64, 58, 911] {
            let spec = spec("cond.sumifs", 1, seed, 1);
            let task = build(&spec).expect("build");
            let c = ctx(&spec);
            let (cell, ..) = live_kind(&task);
            let cell = cell.expect("tier 1 has a cell");
            let Some(Kind::Live { dirty, control, .. }) = task.checks.first().map(|c| c.kind.clone()) else { panic!() };
            let (broken, _) = dirty.first().expect("tier 1 has a broken status");
            let row: u32 = broken[1..].parse().expect("row");
            // As in a real session: supplier and status taken from the broken row itself.
            let f = format!("=SUMIFS(G{0}:G{1},C{0}:C{1},C{row},H{0}:H{1},{broken})", c.first, c.last);
            let mut s = task.session().expect("session");
            sheet::set(&mut s, &cell, &Cell::Formula(f)).expect("answer");
            let fb = evaluate(&task, &s, &[]).expect("evaluate").outcomes[0].feedback.clone();
            assert!(fb.contains(&format!("Условие взято из {broken}")), "s{seed}: {fb}");

            assert!(control.contains("должно дать 0"), "{control}");
            sheet::set(&mut s, &cell, &Cell::Formula(format!("=G{}*0+7", c.first))).expect("answer");
            let fb = evaluate(&task, &s, &[]).expect("evaluate").outcomes[0].feedback.clone();
            assert!(fb.contains("Проверь себя формулой"), "s{seed}: {fb}");
        }
    }

    #[test]
    fn every_control_formula_is_zero_on_a_cleaned_table() {
        // The control formula must really prove cleanliness: 0 (or no mismatch) once the dirt is gone.
        let mut checked = 0;
        for skill in ["fn.sum", "cond.sumifs", "fn.count"] {
            for tier in 1..=2u8 {
                let spec = spec(skill, tier, 58, 1);
                let task = build(&spec).expect("build");
                let c = ctx(&spec);
                let Some(Kind::Live { control, .. }) = task.checks.first().map(|c| c.kind.clone()) else { panic!() };
                let mut s = task.session().expect("session");
                clean_by_hand(&mut s, &c);
                for part in control.split("; ").filter(|p| p.starts_with('=')) {
                    let Some((formula, rest)) = part.split_once(' ') else { continue };
                    if !rest.starts_with("должно дать 0") {
                        continue;
                    }
                    let canonical =
                        gridcraft_engine::formula::parse_input(formula.trim_start_matches('='), gridcraft_engine::formula::FormulaLocale::Ru, |_| {
                            false
                        })
                        .and_then(|e| gridcraft_engine::formula::print_input(&e, gridcraft_engine::formula::FormulaLocale::En, |_| false))
                        .expect("parse control");
                    sheet::set(&mut s, "ZZ1", &Cell::Formula(format!("={canonical}"))).expect("control");
                    let v = sheet::value(&s, "ZZ1").expect("value");
                    assert!(matches!(v, Value::Number(n) if n.abs() < 0.005), "{skill} t{tier}: {formula} = {v:?}");
                    checked += 1;
                }
            }
        }
        assert!(checked >= 4, "only {checked} control formulas evaluated");
    }

    #[test]
    fn typed_numbers_and_constant_formulas_are_not_answers() {
        let spec = spec("cond.sumif", 1, 42, 1);
        let task = build(&spec).expect("build");
        let (cell, _, expected, _) = live_kind(&task);
        let cell = cell.expect("tier 1 has a cell");
        for input in [Cell::Num(expected), Cell::Formula(format!("={expected}"))] {
            let mut s = task.session().expect("session");
            sheet::set(&mut s, &cell, &input).expect("set");
            let e = evaluate(&task, &s, &[]).expect("evaluate");
            assert!(!e.passed());
            assert!(e.outcomes.iter().any(|o| o.feedback.contains("вручную")));
        }
    }

    #[test]
    fn a_self_placed_answer_needs_its_label() {
        let spec = spec("fn.sum", 3, 9, 1);
        let task = build(&spec).expect("build");
        let c = ctx(&spec);
        let mut s = task.session().expect("session");
        clean_by_hand(&mut s, &c);
        let f = naive_formula(&c).expect("formula");
        sheet::set(&mut s, "N3", &Cell::Formula(f.clone())).expect("unlabelled");
        let e = evaluate(&task, &s, &[]).expect("evaluate");
        assert!(!e.passed());
        assert!(e.outcomes.iter().any(|o| o.feedback.contains(ANSWER_LABEL)));
        // A label below the table, answer underneath it, also counts.
        let below = c.last + 2;
        sheet::set(&mut s, &format!("B{below}"), &Cell::Text("ответ".into())).expect("label");
        sheet::set(&mut s, &format!("B{}", below + 1), &Cell::Formula(f)).expect("answer");
        assert!(evaluate(&task, &s, &[]).expect("evaluate").passed());
    }

    #[test]
    fn no_two_given_cells_share_an_address() {
        for skill in SKILLS {
            for tier in 1..=TIERS {
                for seed in 1..=20u64 {
                    let task = build(&spec(skill, tier, seed, 1)).expect("build");
                    let mut seen = std::collections::BTreeSet::new();
                    for (cell, ..) in &task.setup.cells {
                        assert!(seen.insert(cell.clone()), "{skill} t{tier} s{seed}: {cell} written twice");
                    }
                    if let Some(Kind::Live { cell: Some(answer), .. }) = task.checks.first().map(|c| &c.kind) {
                        assert!(!seen.contains(answer), "{skill} t{tier} s{seed}: answer cell {answer} is not empty");
                    }
                }
            }
        }
    }

    #[test]
    fn the_register_opens_below_earlier_work_and_is_deterministic() {
        let a = build(&spec("cond.countif", 2, 17, 30)).expect("build");
        let b = build(&spec("cond.countif", 2, 17, 30)).expect("build");
        assert_eq!(serde_json::to_string(&a.setup).ok(), serde_json::to_string(&b.setup).ok());
        assert!(a.setup.cells.iter().all(|(c, ..)| sheet::address(c).is_ok_and(|r| r.row + 1 >= 30)));
        let other = build(&spec("cond.countif", 2, 18, 30)).expect("build");
        assert_ne!(serde_json::to_string(&a.setup).ok(), serde_json::to_string(&other.setup).ok());
        assert!(a.prompt.contains("Андрей"));
        assert!(!a.prompt.contains("СЧЁТЕСЛИ"), "the brief must not name the function");
    }
}
