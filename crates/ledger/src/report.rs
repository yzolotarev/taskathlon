//! Standard reports read straight from the entries: the trial balance ("ОСВ"), the trial balance
//! of one account by subconto, the account card and the account analysis.

use crate::accounts::{self, Kind};
use crate::catalog::Sub;
use crate::money::{Date, Money};
use crate::{Book, Entry};
use gridcraft_tr::ts;
use std::collections::BTreeMap;

/// One row of a trial balance: opening, turnover and closing, split into debit and credit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub code: String,
    pub name: String,
    pub open_dt: Money,
    pub open_kt: Money,
    pub turn_dt: Money,
    pub turn_kt: Money,
    pub close_dt: Money,
    pub close_kt: Money,
}

impl Row {
    fn add(&mut self, o: &Row) {
        self.open_dt += o.open_dt;
        self.open_kt += o.open_kt;
        self.turn_dt += o.turn_dt;
        self.turn_kt += o.turn_kt;
        self.close_dt += o.close_dt;
        self.close_kt += o.close_kt;
    }

    pub fn is_empty(&self) -> bool {
        [self.open_dt, self.open_kt, self.turn_dt, self.turn_kt, self.close_dt, self.close_kt].iter().all(|m| m.is_zero())
    }
}

/// Splits a debit-positive balance into (debit, credit).
fn split(b: Money) -> (Money, Money) {
    if b >= Money::ZERO { (b, Money::ZERO) } else { (Money::ZERO, -b) }
}

/// Per-subconto balances and turnovers of one account for a period.
fn movements(book: &Book, code: &str, from: Date, to: Date) -> BTreeMap<Option<Sub>, (Money, Money, Money)> {
    let mut m: BTreeMap<Option<Sub>, (Money, Money, Money)> = BTreeMap::new();
    for e in &book.entries {
        for (acc, sub, sign) in [(e.dt, e.dt_sub, 1), (e.kt, e.kt_sub, -1)] {
            if acc != code || e.date > to {
                continue;
            }
            let v = m.entry(sub).or_default();
            if e.date < from {
                v.0 += if sign > 0 { e.amount } else { -e.amount };
            } else if sign > 0 {
                v.1 += e.amount;
            } else {
                v.2 += e.amount;
            }
        }
    }
    m
}

fn row_for(book: &Book, code: &str, name: &str, from: Date, to: Date) -> Row {
    let kind = accounts::account(code).map(|a| a.kind).unwrap_or(Kind::ActivePassive);
    let m = movements(book, code, from, to);
    let mut r = Row { code: code.into(), name: name.into(), ..Row::default() };
    // Active-passive accounts with analytics show an expanded balance: debit and credit
    // balances of different counterparties are not netted.
    let expanded = kind == Kind::ActivePassive;
    let (mut open, mut close) = (Money::ZERO, Money::ZERO);
    for (open_b, dt, kt) in m.values() {
        r.turn_dt += *dt;
        r.turn_kt += *kt;
        let close_b = *open_b + *dt - *kt;
        if expanded {
            let (a, b) = split(*open_b);
            r.open_dt += a;
            r.open_kt += b;
            let (a, b) = split(close_b);
            r.close_dt += a;
            r.close_kt += b;
        } else {
            open += *open_b;
            close += close_b;
        }
    }
    if !expanded {
        (r.open_dt, r.open_kt) = split(open);
        (r.close_dt, r.close_kt) = split(close);
    }
    r
}

/// The trial balance ("Оборотно-сальдовая ведомость") for a period. Group accounts (68, 90, 91)
/// are shown as the sum of their subaccounts; the counter account 000 is left out. The last
/// row is the total.
pub fn trial_balance(book: &Book, from: Date, to: Date) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut total = Row { code: String::new(), name: ts("Итого").into(), ..Row::default() };
    for a in accounts::CHART.iter().filter(|a| a.code != "000") {
        if a.group {
            let mut g = Row { code: a.code.into(), name: ts(a.name).into(), ..Row::default() };
            for s in accounts::CHART.iter().filter(|s| s.code.starts_with(&format!("{}.", a.code))) {
                g.add(&row_for(book, s.code, ts(s.name), from, to));
            }
            // A group shows the synthetic balance: its subaccounts net out (90 closes to zero).
            (g.open_dt, g.open_kt) = split(g.open_dt - g.open_kt);
            (g.close_dt, g.close_kt) = split(g.close_dt - g.close_kt);
            if !g.is_empty() {
                rows.push(g);
            }
            continue;
        }
        let r = row_for(book, a.code, ts(a.name), from, to);
        if r.is_empty() {
            continue;
        }
        total.add(&r);
        rows.push(r);
    }
    rows.push(total);
    rows
}

/// The trial balance of one account broken down by subconto, with quantities for stock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubRow {
    pub sub: Option<Sub>,
    pub name: String,
    pub row: Row,
    pub qty: i64,
}

pub fn account_balance(book: &Book, code: &str, from: Date, to: Date) -> Vec<SubRow> {
    let quantity = accounts::account(code).is_some_and(|a| a.quantity);
    let mut out = Vec::new();
    for (sub, (open_b, dt, kt)) in movements(book, code, from, to) {
        let (open_dt, open_kt) = split(open_b);
        let (close_dt, close_kt) = split(open_b + dt - kt);
        let row = Row { code: code.into(), name: String::new(), open_dt, open_kt, turn_dt: dt, turn_kt: kt, close_dt, close_kt };
        let qty = if quantity { quantity_on(book, code, sub, to) } else { 0 };
        if row.is_empty() && qty == 0 {
            continue;
        }
        let name = sub.map(|s| book.catalogs.sub_name(s)).unwrap_or_else(|| ts("<без аналитики>").into());
        out.push(SubRow { sub, name, row, qty });
    }
    out
}

/// Quantity on a stock account at the end of `to`.
pub fn quantity_on(book: &Book, code: &str, sub: Option<Sub>, to: Date) -> i64 {
    book.entries
        .iter()
        .filter(|e| e.date <= to)
        .map(|e| if e.dt == code && e.dt_sub == sub { e.qty } else { 0 } - if e.kt == code && e.kt_sub == sub { e.qty } else { 0 })
        .sum()
}

/// Balance of an account (all subcontos) at the end of `to`, debit-positive.
pub fn balance(book: &Book, code: &str, to: Date) -> Money {
    book.entries
        .iter()
        .filter(|e| e.date <= to)
        .map(|e| {
            let dt = e.dt == code || e.dt.starts_with(&format!("{code}."));
            let kt = e.kt == code || e.kt.starts_with(&format!("{code}."));
            (if dt { e.amount } else { Money::ZERO }) - (if kt { e.amount } else { Money::ZERO })
        })
        .sum()
}

/// A line of the account card ("Карточка счёта").
#[derive(Clone, Debug)]
pub struct CardLine {
    pub date: Date,
    pub doc: u32,
    pub document: String,
    pub memo: String,
    pub corr: &'static str,
    pub sub: Option<Sub>,
    pub dt: Money,
    pub kt: Money,
    /// Running balance after this line, debit-positive.
    pub balance: Money,
}

fn touches(e: &Entry, code: &str, sub: Option<Sub>) -> (bool, bool) {
    let hit = |acc: &str, s: Option<Sub>| (acc == code || acc.starts_with(&format!("{code}."))) && (sub.is_none() || s == sub);
    (hit(e.dt, e.dt_sub), hit(e.kt, e.kt_sub))
}

/// Every entry on an account (optionally one subconto) in the period, with a running balance.
/// Returns the opening balance and the lines.
pub fn account_card(book: &Book, code: &str, sub: Option<Sub>, from: Date, to: Date) -> (Money, Vec<CardLine>) {
    let mut entries: Vec<&Entry> = book.entries.iter().filter(|e| e.date <= to).collect();
    entries.sort_by_key(|e| e.date);
    let mut balance = Money::ZERO;
    let mut lines = Vec::new();
    let mut opening = Money::ZERO;
    for e in entries {
        let (dt, kt) = touches(e, code, sub);
        if !dt && !kt {
            continue;
        }
        let delta = if dt { e.amount } else { Money::ZERO } - if kt { e.amount } else { Money::ZERO };
        balance += delta;
        if e.date < from {
            opening = balance;
            continue;
        }
        let document = book.doc(e.doc).map(|d| d.label()).unwrap_or_default();
        let (corr, corr_sub) = if dt { (e.kt, e.kt_sub) } else { (e.dt, e.dt_sub) };
        lines.push(CardLine {
            date: e.date,
            doc: e.doc,
            document,
            memo: e.memo.clone(),
            corr,
            sub: if dt { e.dt_sub } else { e.kt_sub }.or(corr_sub),
            dt: if dt { e.amount } else { Money::ZERO },
            kt: if kt { e.amount } else { Money::ZERO },
            balance,
        });
    }
    (opening, lines)
}

/// Account analysis ("Анализ счёта"): turnover with each corresponding account.
pub fn account_analysis(book: &Book, code: &str, from: Date, to: Date) -> Vec<(&'static str, Money, Money)> {
    let mut m: BTreeMap<&'static str, (Money, Money)> = BTreeMap::new();
    for e in book.entries.iter().filter(|e| e.date >= from && e.date <= to) {
        let (dt, kt) = touches(e, code, None);
        if dt && !kt {
            m.entry(e.kt).or_default().0 += e.amount;
        } else if kt && !dt {
            m.entry(e.dt).or_default().1 += e.amount;
        }
    }
    m.into_iter().map(|(k, (a, b))| (k, a, b)).collect()
}

/// Revenue, costs and profit for a period, read from 90 and 91 subaccounts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Results {
    pub revenue: Money,
    pub vat: Money,
    pub cost_of_sales: Money,
    pub selling: Money,
    pub admin: Money,
    pub sales_profit: Money,
    pub other_income: Money,
    pub other_expense: Money,
    pub before_tax: Money,
    pub tax: Money,
    pub net: Money,
}

pub fn results(book: &Book, from: Date, to: Date) -> Results {
    let turn = |code: &str, debit: bool| -> Money {
        book.entries.iter().filter(|e| e.date >= from && e.date <= to && if debit { e.dt == code } else { e.kt == code }).map(|e| e.amount).sum()
    };
    let mut r = Results {
        revenue: turn("90.01", false),
        vat: turn("90.03", true),
        cost_of_sales: turn("90.02", true),
        selling: turn("90.07", true),
        admin: turn("90.08", true),
        other_income: turn("91.01", false),
        other_expense: turn("91.02", true),
        tax: turn("68.04", false),
        ..Results::default()
    };
    r.sales_profit = r.revenue - r.vat - r.cost_of_sales - r.selling - r.admin;
    r.before_tax = r.sales_profit + r.other_income - r.other_expense;
    r.net = r.before_tax - r.tax;
    r
}
