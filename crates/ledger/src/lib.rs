//! Taskathlon's accounting core, written clean-room from public accounting rules (the Russian
//! chart of accounts, PBU/FSBU and the Tax Code). Catalogs hold who and what; documents are the
//! only input; posting a document produces double entries; reports read the entries.
//!
//! Every posted document is re-posted in date order whenever anything changes, so average
//! cost and month closing always reflect the current state of the books.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable)]

use gridcraft_tr::{t as tr, ts};
pub mod accounts;
pub mod catalog;
pub mod demo;
pub mod document;
pub mod money;
pub mod practice;
pub mod report;
#[cfg(test)]
mod tests;

pub use accounts::{Account, Analytic, CHART, account};
pub use catalog::{Catalogs, Sub, Vat};
pub use document::{Body, DocKind, Document};
pub use money::{Date, Money};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Personal income tax withheld from salary, percent (base rate; the progressive scale above
/// 2.4 million a year is not modelled).
pub const NDFL_PERCENT: i64 = 13;
/// Insurance contributions on salary, percent (single base rate, simplified).
pub const CONTRIBUTIONS_PERCENT: i64 = 30;
/// Corporate profit tax, percent (from 2025).
pub const PROFIT_TAX_PERCENT: i64 = 25;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error("документ не найден")]
    NoDocument,
    #[error("{0}")]
    Invalid(String),
}

/// One double entry ("проводка").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub doc: u32,
    pub date: Date,
    pub dt: &'static str,
    pub dt_sub: Option<Sub>,
    pub kt: &'static str,
    pub kt_sub: Option<Sub>,
    pub amount: Money,
    /// Quantity, for stock accounts on either side.
    pub qty: i64,
    pub memo: String,
}

/// Running balances while posting: debit-positive amount and quantity per (account, subconto).
#[derive(Default)]
struct Balances(BTreeMap<(&'static str, Option<Sub>), (Money, i64)>);

impl Balances {
    fn apply(&mut self, e: &Entry) {
        let dt = self.0.entry((e.dt, e.dt_sub)).or_default();
        dt.0 += e.amount;
        if accounts::account(e.dt).is_some_and(|a| a.quantity) {
            dt.1 = dt.1.saturating_add(e.qty);
        }
        let kt = self.0.entry((e.kt, e.kt_sub)).or_default();
        kt.0 -= e.amount;
        if accounts::account(e.kt).is_some_and(|a| a.quantity) {
            kt.1 = kt.1.saturating_sub(e.qty);
        }
    }

    fn get(&self, acc: &str, sub: Option<Sub>) -> (Money, i64) {
        self.0.iter().find(|((a, s), _)| *a == acc && *s == sub).map(|(_, v)| *v).unwrap_or_default()
    }

    /// Total over every subconto of an account.
    fn total(&self, acc: &str) -> Money {
        self.0.iter().filter(|((a, _), _)| *a == acc).map(|(_, v)| v.0).sum()
    }

    fn subs(&self, acc: &str) -> Vec<(Option<Sub>, Money, i64)> {
        self.0.iter().filter(|((a, _), _)| *a == acc).map(|((_, s), v)| (*s, v.0, v.1)).collect()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Book {
    pub org: String,
    pub catalogs: Catalogs,
    pub docs: Vec<Document>,
    /// Entries of every posted document, in posting order. Rebuilt by [`Book::repost`].
    #[serde(skip)]
    pub entries: Vec<Entry>,
    /// Problems found while posting (document id, message), e.g. selling more than in stock.
    #[serde(skip)]
    pub problems: Vec<(u32, String)>,
}

fn key(acc: &str) -> Result<&'static str, String> {
    match accounts::account(acc) {
        Some(a) if !a.group => Ok(a.code),
        Some(a) => Err(tr(&format!("Счёт {} — группа, выберите субсчёт", a.code))),
        None => Err(tr(&format!("Нет счёта {acc} в плане счетов"))),
    }
}

/// Builds entries for one document.
struct Poster<'a> {
    doc: &'a Document,
    book: &'a Book,
    out: Vec<Entry>,
    problems: Vec<String>,
}

impl Poster<'_> {
    #[allow(clippy::too_many_arguments)]
    fn e(&mut self, dt: &str, dt_sub: Option<Sub>, kt: &str, kt_sub: Option<Sub>, amount: Money, qty: i64, memo: &str) -> Result<(), String> {
        if amount.is_zero() && qty == 0 {
            return Ok(());
        }
        let (dt, kt) = (key(dt)?, key(kt)?);
        for (code, sub) in [(dt, dt_sub), (kt, kt_sub)] {
            let want = accounts::account(code).map(|a| a.analytic).unwrap_or(Analytic::None);
            let ok = matches!(
                (want, sub),
                (_, None)
                    | (Analytic::Counterparty, Some(Sub::Counterparty(_)))
                    | (Analytic::Item, Some(Sub::Item(_)))
                    | (Analytic::Employee, Some(Sub::Employee(_)))
                    | (Analytic::Asset, Some(Sub::Asset(_)))
            );
            if !ok {
                return Err(tr(&format!("Счёт {code}: неверная аналитика")));
            }
        }
        self.out.push(Entry { doc: self.doc.id, date: self.doc.date, dt, dt_sub, kt, kt_sub, amount, qty, memo: memo.into() });
        Ok(())
    }

    fn cp(&self, id: u32) -> Result<Option<Sub>, String> {
        self.book.catalogs.counterparty(id).map(|_| Some(Sub::Counterparty(id))).ok_or_else(|| ts("Не выбран контрагент").to_string())
    }

    fn emp(&self, id: u32) -> Result<Option<Sub>, String> {
        self.book.catalogs.employee(id).map(|_| Some(Sub::Employee(id))).ok_or_else(|| ts("Не выбран сотрудник").to_string())
    }

    fn item_vat(&self, id: u32) -> Result<Vat, String> {
        self.book.catalogs.item(id).map(|i| i.vat).ok_or_else(|| ts("Не выбрана номенклатура").to_string())
    }

    /// Average cost of `qty` units of `item` on `acc`; notes a problem when stock is short.
    fn cost(&mut self, bal: &Balances, acc: &str, item: u32, qty: i64) -> Money {
        let (amount, have) = bal.get(acc, Some(Sub::Item(item)));
        let name = self.book.catalogs.item(item).map(|i| i.name.clone()).unwrap_or_default();
        if qty > have {
            self.problems.push(tr(&format!("Не хватает «{name}» на счёте {acc}: нужно {qty}, есть {have}")));
        }
        if have <= 0 {
            return Money::ZERO;
        }
        if qty >= have { amount } else { amount.mul_div(qty, have) }
    }

    fn post(&mut self, bal: &Balances, month: &[Entry]) -> Result<(), String> {
        let d = self.doc;
        match &d.body {
            Body::Opening { lines } => {
                for l in lines {
                    self.e(&l.account, l.sub, "000", None, l.debit, l.qty, ts("Ввод остатков"))?;
                    self.e("000", None, &l.account, l.sub, l.credit, if l.debit.is_zero() { l.qty } else { 0 }, ts("Ввод остатков"))?;
                }
            }
            Body::Receipt { supplier, account, lines } => {
                if account != "41" && account != "10" {
                    return Err(ts("Товары приходуются на 41, материалы на 10").into());
                }
                let s = self.cp(*supplier)?;
                for l in lines {
                    let vat = self.item_vat(l.item)?.inside(l.amount);
                    self.e(account, Some(Sub::Item(l.item)), "60", s, l.amount - vat, l.qty, ts("Поступление"))?;
                    self.e("19", s, "60", s, vat, 0, ts("НДС предъявлен поставщиком"))?;
                }
            }
            Body::Services { supplier, lines } => {
                let s = self.cp(*supplier)?;
                for l in lines {
                    let vat = l.vat.inside(l.amount);
                    self.e(&l.account, None, "60", s, l.amount - vat, 0, &l.text)?;
                    self.e("19", s, "60", s, vat, 0, ts("НДС предъявлен поставщиком"))?;
                }
            }
            Body::Sale { customer, lines } => {
                let c = self.cp(*customer)?;
                for l in lines {
                    let vat = self.item_vat(l.item)?.inside(l.amount);
                    let cost = self.cost(bal, "41", l.item, l.qty);
                    self.e("62", c, "90.01", None, l.amount, 0, ts("Выручка"))?;
                    self.e("90.03", None, "68.02", None, vat, 0, ts("НДС с выручки"))?;
                    self.e("90.02", None, "41", Some(Sub::Item(l.item)), cost, l.qty, ts("Себестоимость"))?;
                }
            }
            Body::AssetPurchase { supplier, asset, amount, vat } => {
                let s = self.cp(*supplier)?;
                let a = self.book.catalogs.asset(*asset).map(|_| Some(Sub::Asset(*asset))).ok_or(ts("Не выбрано основное средство"))?;
                let v = vat.inside(*amount);
                self.e("08", a, "60", s, *amount - v, 0, ts("Покупка ОС"))?;
                self.e("19", s, "60", s, v, 0, ts("НДС предъявлен поставщиком"))?;
                self.e("01", a, "08", a, *amount - v, 0, ts("Ввод в эксплуатацию"))?;
            }
            Body::WriteOff { account, lines } => {
                for l in lines {
                    let cost = self.cost(bal, "10", l.item, l.qty);
                    self.e(account, None, "10", Some(Sub::Item(l.item)), cost, l.qty, ts("Списание материалов"))?;
                }
            }
            Body::BankIn { from, amount } => match from {
                document::BankIn::Customer(c) => self.e("51", None, "62", self.cp(*c)?, *amount, 0, ts("Оплата от покупателя"))?,
                document::BankIn::Loan(c) => self.e("51", None, "66", self.cp(*c)?, *amount, 0, ts("Получен заём"))?,
                document::BankIn::FromCash => self.e("51", None, "50", None, *amount, 0, ts("Взнос наличных"))?,
            },
            Body::BankOut { to, amount } => {
                if bal.total("51") < *amount {
                    self.problems.push(tr(&format!("На расчётном счёте не хватает денег: остаток {}", bal.total("51"))));
                }
                match to {
                    document::BankOut::Supplier(c) => self.e("60", self.cp(*c)?, "51", None, *amount, 0, ts("Оплата поставщику"))?,
                    document::BankOut::Tax(acc) => {
                        if !acc.starts_with("68.") {
                            return Err(ts("Налог платится со счёта 68.xx").into());
                        }
                        self.e(acc, None, "51", None, *amount, 0, ts("Уплата налога"))?
                    }
                    document::BankOut::Insurance => self.e("69", None, "51", None, *amount, 0, ts("Уплата взносов"))?,
                    document::BankOut::Salary(e) => self.e("70", self.emp(*e)?, "51", None, *amount, 0, ts("Выплата зарплаты"))?,
                    document::BankOut::Fee => self.e("91.02", None, "51", None, *amount, 0, ts("Комиссия банка"))?,
                    document::BankOut::LoanRepay(c) => self.e("66", self.cp(*c)?, "51", None, *amount, 0, ts("Возврат займа"))?,
                    document::BankOut::ToCash => self.e("50", None, "51", None, *amount, 0, ts("Снятие наличных в кассу"))?,
                }
            }
            Body::CashIn { from, amount } => match from {
                document::CashIn::Customer(c) => self.e("50", None, "62", self.cp(*c)?, *amount, 0, ts("Оплата наличными"))?,
                document::CashIn::AdvanceReturn(e) => self.e("50", None, "71", self.emp(*e)?, *amount, 0, ts("Возврат подотчётных"))?,
            },
            Body::CashOut { to, amount } => {
                if bal.total("50") < *amount {
                    self.problems.push(tr(&format!("В кассе не хватает денег: остаток {}", bal.total("50"))));
                }
                match to {
                    document::CashOut::Advance(e) => self.e("71", self.emp(*e)?, "50", None, *amount, 0, ts("Выдано под отчёт"))?,
                    document::CashOut::Salary(e) => self.e("70", self.emp(*e)?, "50", None, *amount, 0, ts("Выплата зарплаты"))?,
                    document::CashOut::Supplier(c) => self.e("60", self.cp(*c)?, "50", None, *amount, 0, ts("Оплата поставщику"))?,
                }
            }
            Body::AdvanceReport { employee, lines } => {
                let e = self.emp(*employee)?;
                for l in lines {
                    let vat = l.vat.inside(l.amount);
                    self.e(&l.account, None, "71", e, l.amount - vat, 0, &l.text)?;
                    self.e("19", None, "71", e, vat, 0, ts("НДС по авансовому отчёту"))?;
                }
            }
            Body::Payroll { account, lines } => {
                for (id, gross) in lines {
                    let e = self.emp(*id)?;
                    self.e(account, None, "70", e, *gross, 0, ts("Начислена зарплата"))?;
                    self.e("70", e, "68.01", None, gross.mul_div(NDFL_PERCENT, 100), 0, ts("Удержан НДФЛ"))?;
                    self.e(account, None, "69", None, gross.mul_div(CONTRIBUTIONS_PERCENT, 100), 0, ts("Начислены страховые взносы"))?;
                }
            }
            Body::MonthClose => self.close(bal, month)?,
        }
        Ok(())
    }

    fn close(&mut self, bal: &Balances, month: &[Entry]) -> Result<(), String> {
        let d = self.doc.date;
        // 1. Depreciation: straight line from the month after the asset went into service.
        for (sub, cost, _) in bal.subs("01") {
            let Some(Sub::Asset(id)) = sub else { continue };
            let Some(asset) = self.book.catalogs.asset(id) else { continue };
            let start = self.book.entries.iter().filter(|e| e.dt == "01" && e.dt_sub == sub).map(|e| e.date).min();
            if start.is_none_or(|s| s.months_until(d) < 1) || cost <= Money::ZERO {
                continue;
            }
            let initial: Money = self.book.entries.iter().filter(|e| e.dt == "01" && e.dt_sub == sub).map(|e| e.amount).sum();
            let left = cost + bal.get("02", sub).0;
            let amount = initial.mul_div(1, i64::from(asset.life_months)).min(left);
            self.e("26", None, "02", sub, amount, 0, ts("Амортизация"))?;
        }
        // 2. VAT deduction for input VAT.
        for (sub, amount, _) in bal.subs("19") {
            if amount > Money::ZERO {
                self.e("68.02", None, "19", sub, amount, 0, ts("Вычет НДС"))?;
            }
        }
        // Balances after depreciation (it adds to 26).
        let mut after = Balances(bal.0.clone());
        for e in &self.out {
            after.apply(e);
        }
        // 3. Close cost accounts into sales.
        for (from, to, memo) in
            [("20", "90.02", ts("Закрытие счёта 20")), ("26", "90.08", ts("Закрытие счёта 26")), ("44", "90.07", ts("Закрытие счёта 44"))]
        {
            let amount = after.total(from);
            if amount > Money::ZERO {
                self.e(to, None, from, None, amount, 0, memo)?;
            }
        }
        // 4–5. Financial result of the month on 90 and 91.
        let in_month = |e: &&Entry| e.date.y == d.y && e.date.m == d.m;
        let flow = |codes: &[&str], entries: &[Entry]| -> Money {
            entries
                .iter()
                .filter(in_month)
                .map(|e| if codes.contains(&e.kt) { e.amount } else { Money::ZERO } - if codes.contains(&e.dt) { e.amount } else { Money::ZERO })
                .sum()
        };
        let mut so_far: Vec<Entry> = month.to_vec();
        so_far.extend(self.out.iter().cloned());
        let sales = flow(&["90.01", "90.02", "90.03", "90.07", "90.08"], &so_far);
        let other = flow(&["91.01", "91.02"], &so_far);
        for (acc, result, memo) in [("90.09", sales, ts("Финансовый результат от продаж")), ("91.09", other, ts("Сальдо прочих доходов и расходов"))]
        {
            if result > Money::ZERO {
                self.e(acc, None, "99", None, result, 0, memo)?;
            } else if result < Money::ZERO {
                self.e("99", None, acc, None, -result, 0, memo)?;
            }
        }
        // 6. Profit tax on a profitable month (simplified: no deferred tax).
        let profit = sales + other;
        if profit > Money::ZERO {
            self.e("99", None, "68.04", None, profit.mul_div(PROFIT_TAX_PERCENT, 100), 0, ts("Налог на прибыль"))?;
        }
        Ok(())
    }
}

impl Book {
    pub fn new(org: &str) -> Book {
        Book { org: org.into(), ..Book::default() }
    }

    pub fn doc(&self, id: u32) -> Option<&Document> {
        self.docs.iter().find(|d| d.id == id)
    }

    pub fn doc_mut(&mut self, id: u32) -> Option<&mut Document> {
        self.docs.iter_mut().find(|d| d.id == id)
    }

    /// Adds an unposted document with the next number of its kind.
    pub fn add(&mut self, date: Date, body: Body) -> u32 {
        let id = self.docs.iter().map(|d| d.id).max().unwrap_or(0).saturating_add(1);
        let kind = body.kind();
        let n = self.docs.iter().filter(|d| d.body.kind() == kind).count() + 1;
        let number = format!("{}-{:05}", kind.prefix(), n);
        self.docs.push(Document { id, number, date, posted: false, body, comment: String::new() });
        id
    }

    /// Adds and posts in one step.
    pub fn add_posted(&mut self, date: Date, body: Body) -> Result<u32, LedgerError> {
        let id = self.add(date, body);
        self.post(id)?;
        Ok(id)
    }

    /// Posts a document. Fails, leaving it unposted, if it can't be posted cleanly.
    pub fn post(&mut self, id: u32) -> Result<(), LedgerError> {
        let doc = self.doc_mut(id).ok_or(LedgerError::NoDocument)?;
        let was = doc.posted;
        doc.posted = true;
        let result = self.repost();
        let problem = self.problems.iter().find(|p| p.0 == id).map(|p| p.1.clone());
        match (result, problem) {
            (Ok(()), None) => Ok(()),
            (Err(e), _) | (Ok(()), Some(e)) => {
                if let Some(doc) = self.doc_mut(id) {
                    doc.posted = was;
                }
                let _ = self.repost();
                Err(LedgerError::Invalid(e))
            }
        }
    }

    /// Unposts a document: its entries disappear.
    pub fn unpost(&mut self, id: u32) -> Result<(), LedgerError> {
        self.doc_mut(id).ok_or(LedgerError::NoDocument)?.posted = false;
        let _ = self.repost();
        Ok(())
    }

    /// Deletes an unposted document.
    pub fn remove(&mut self, id: u32) -> Result<(), LedgerError> {
        match self.doc(id) {
            None => Err(LedgerError::NoDocument),
            Some(d) if d.posted => Err(LedgerError::Invalid(ts("Сначала отмените проведение").into())),
            Some(_) => {
                self.docs.retain(|d| d.id != id);
                Ok(())
            }
        }
    }

    /// Re-posts every posted document in date order. Returns the first hard error (a document
    /// that can't produce entries at all); soft problems go to `problems`.
    pub fn repost(&mut self) -> Result<(), String> {
        let mut order: Vec<&Document> = self.docs.iter().filter(|d| d.posted).collect();
        order.sort_by_key(|d| (d.date, d.body.kind().order(), d.id));
        let order: Vec<u32> = order.iter().map(|d| d.id).collect();
        self.entries.clear();
        self.problems.clear();
        let mut bal = Balances::default();
        let mut first_error = None;
        for id in order {
            let Some(doc) = self.doc(id) else { continue };
            let month: Vec<Entry> = self.entries.iter().filter(|e| e.date.y == doc.date.y && e.date.m == doc.date.m).cloned().collect();
            let mut p = Poster { doc, book: self, out: Vec::new(), problems: Vec::new() };
            let result = p.post(&bal, &month);
            let (out, problems) = (p.out, p.problems);
            match result {
                Ok(()) => {
                    for e in &out {
                        bal.apply(e);
                    }
                    self.entries.extend(out);
                    self.problems.extend(problems.into_iter().map(|m| (id, m)));
                }
                Err(e) => {
                    self.problems.push((id, e.clone()));
                    first_error.get_or_insert(e);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    pub fn entries_of(&self, id: u32) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(move |e| e.doc == id)
    }

    pub fn to_json(&self) -> Result<String, LedgerError> {
        serde_json::to_string_pretty(self).map_err(|e| LedgerError::Invalid(e.to_string()))
    }

    /// Loads a saved book and re-posts it.
    pub fn from_json(s: &str) -> Result<Book, LedgerError> {
        let mut b: Book = serde_json::from_str(s).map_err(|e| LedgerError::Invalid(e.to_string()))?;
        let _ = b.repost();
        Ok(b)
    }
}
