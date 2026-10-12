//! Practice tasks on top of the books: "do this in the program" and "read this from a report".
//! Each task is checked against the book itself, so any correct way of doing it passes.

use crate::catalog::Sub;
use crate::document::{self, Body};
use crate::money::{Date, Money};
use crate::{Book, DocKind, report};

#[derive(Clone, Debug)]
pub enum Check {
    /// A posted document of this kind in the month whose details pass the filter.
    Posted { kind: DocKind, month: (u16, u8), want: Want },
    /// Balance of an account (optionally one subconto) at a date, debit-positive.
    Balance { account: &'static str, sub: Option<Sub>, at: Date, want: Money },
    /// Each account has a zero balance at the date.
    Closed { accounts: &'static [&'static str], at: Date },
    /// A number read from the books: the credit (or debit) balance of an account at a date.
    ReadBalance { account: &'static str, at: Date, credit: bool },
    /// A number read from the books: profit (loss) from sales for the month.
    ReadSalesProfit { month: (u16, u8) },
}

/// What a posted document must contain.
#[derive(Clone, Debug, Default)]
pub struct Want {
    pub party: Option<u32>,
    pub employee: Option<u32>,
    pub item: Option<u32>,
    pub qty: Option<i64>,
    pub amount: Option<Money>,
    pub account: Option<&'static str>,
    pub lines: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub title: String,
    pub text: String,
    pub hint: String,
    pub check: Check,
    /// Par time for a practised accountant, seconds.
    pub goal_seconds: u32,
}

impl Task {
    /// Whether the task needs a typed number rather than work in the program.
    pub fn asks_number(&self) -> bool {
        matches!(self.check, Check::ReadBalance { .. } | Check::ReadSalesProfit { .. })
    }

    /// The expected typed answer, for read-the-report tasks.
    pub fn expected(&self, book: &Book) -> Option<Money> {
        match &self.check {
            Check::ReadBalance { account, at, credit } => {
                let b = report::balance(book, account, *at);
                Some(if *credit { -b } else { b })
            }
            Check::ReadSalesProfit { month } => {
                let from = Date { y: month.0, m: month.1, d: 1 };
                Some(report::results(book, from, from.month_end()).sales_profit)
            }
            _ => None,
        }
    }

    /// `Ok` when done; otherwise what is still missing.
    pub fn check(&self, book: &Book, answer: Option<Money>) -> Result<(), String> {
        match &self.check {
            Check::Posted { kind, month, want } => {
                let found = book.docs.iter().filter(|d| d.posted && d.body.kind() == *kind && (d.date.y, d.date.m) == *month);
                let mut nearest = None;
                for d in found {
                    match matches(&d.body, want) {
                        Ok(()) => return Ok(()),
                        Err(e) => nearest = Some(e),
                    }
                }
                Err(nearest.unwrap_or_else(|| format!("Нет проведённого документа «{}» за этот месяц", kind.title())))
            }
            Check::Balance { account, sub, at, want } => {
                let got = match sub {
                    None => report::balance(book, account, *at),
                    Some(s) => report::account_balance(book, account, *at, *at)
                        .iter()
                        .find(|r| r.sub == Some(*s))
                        .map(|r| r.row.close_dt - r.row.close_kt)
                        .unwrap_or_default(),
                };
                if got == *want { Ok(()) } else { Err(format!("Сальдо счёта {account} на {at}: {got}, а нужно {want}")) }
            }
            Check::Closed { accounts, at } => {
                if !book.docs.iter().any(|d| d.posted && d.body.kind() == DocKind::MonthClose && d.date.y == at.y && d.date.m == at.m) {
                    return Err("Нет проведённого закрытия месяца".into());
                }
                for a in *accounts {
                    let b = report::balance(book, a, *at);
                    if !b.is_zero() {
                        return Err(format!("На {at} у счёта {a} остался остаток {b}"));
                    }
                }
                Ok(())
            }
            Check::ReadBalance { .. } | Check::ReadSalesProfit { .. } => {
                let want = self.expected(book).unwrap_or_default();
                match answer {
                    Some(a) if a == want => Ok(()),
                    Some(a) => Err(format!("{a} — не сходится. Сверься с отчётом")),
                    None => Err("Введи число".into()),
                }
            }
        }
    }
}

fn matches(body: &Body, w: &Want) -> Result<(), String> {
    let total = body.total();
    if let Some(a) = w.amount
        && total != a
    {
        return Err(format!("Сумма документа {total}, а нужно {a}"));
    }
    let party_ok = |id: u32| w.party.is_none_or(|p| p == id);
    let emp_ok = |id: u32| w.employee.is_none_or(|p| p == id);
    let goods = |lines: &[document::GoodsLine]| -> Result<(), String> {
        if w.item.is_none() && w.qty.is_none() {
            return Ok(());
        }
        let ok = lines.iter().any(|l| w.item.is_none_or(|i| i == l.item) && w.qty.is_none_or(|q| q == l.qty));
        if ok { Ok(()) } else { Err("Номенклатура или количество не те".into()) }
    };
    match body {
        Body::Receipt { supplier, account, lines } => {
            if !party_ok(*supplier) {
                return Err("Не тот поставщик".into());
            }
            if w.account.is_some_and(|a| a != account) {
                return Err("Не тот счёт учёта".into());
            }
            goods(lines)
        }
        Body::Sale { customer, lines } => {
            if !party_ok(*customer) {
                return Err("Не тот покупатель".into());
            }
            goods(lines)
        }
        Body::Services { supplier, lines } => {
            if !party_ok(*supplier) {
                return Err("Не тот поставщик".into());
            }
            if let Some(a) = w.account
                && !lines.iter().all(|l| l.account == a)
            {
                return Err(format!("Затраты должны идти на счёт {a}"));
            }
            Ok(())
        }
        Body::BankIn { from, .. } => match from {
            document::BankIn::Customer(c) | document::BankIn::Loan(c) if party_ok(*c) => Ok(()),
            _ => Err("Не тот плательщик или вид операции".into()),
        },
        Body::BankOut { to, .. } => match to {
            document::BankOut::Supplier(c) | document::BankOut::LoanRepay(c) if party_ok(*c) && w.account.is_none() => Ok(()),
            document::BankOut::Tax(a) if w.account == Some(a.as_str()) => Ok(()),
            document::BankOut::Salary(e) if emp_ok(*e) && w.employee.is_some() => Ok(()),
            _ => Err("Не тот получатель или вид операции".into()),
        },
        Body::CashOut { to, .. } => match to {
            document::CashOut::Advance(e) if emp_ok(*e) => Ok(()),
            _ => Err("Нужна выдача под отчёт нужному сотруднику".into()),
        },
        Body::CashIn { .. }
        | Body::AdvanceReport { .. }
        | Body::AssetPurchase { .. }
        | Body::WriteOff { .. }
        | Body::Opening { .. }
        | Body::MonthClose => Ok(()),
        Body::Payroll { account, lines } => {
            if w.account.is_some_and(|a| a != account) {
                return Err("Не тот счёт затрат".into());
            }
            if w.lines.is_some_and(|n| lines.len() < n) {
                return Err("Начислены не все сотрудники".into());
            }
            Ok(())
        }
    }
}

/// A small deterministic generator so every run has different numbers.
struct Lcg(u64);

impl Lcg {
    fn int(&mut self, lo: i64, hi: i64) -> i64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let span = (hi - lo + 1).max(1) as u64;
        lo + ((self.0 >> 33) % span) as i64
    }
}

/// The February workshop on top of the demo company's January books.
pub fn february(seed: u64) -> Vec<Task> {
    let mut r = Lcg(seed ^ 0x9e37_79b9_7f4a_7c15);
    let feb = (2026, 2);
    let end = Date { y: 2026, m: 2, d: 28 };
    let q_in = r.int(5, 15) * 10;
    let price_in = r.int(30, 42) * 10;
    let q_out = r.int(3, 8) * 10;
    let price_out = r.int(55, 70) * 10;
    let advance = r.int(2, 6) * 500;
    vec![
        Task {
            title: "Поступление товаров".into(),
            text: format!("От ООО «Бумажный двор» пришли {q_in} пачек бумаги А4 на {} ₽ с НДС. Оформи поступление на счёт 41 в феврале и проведи.", Money::rub(q_in * price_in)),
            hint: "📒 ⬇ Закупки → «+ Поступление товаров и материалов». Поставщик, номенклатура, количество, сумма → «Провести и закрыть».".into(),
            check: Check::Posted { kind: DocKind::Receipt, month: feb, want: Want { party: Some(1), item: Some(1), qty: Some(q_in), amount: Some(Money::rub(q_in * price_in)), account: Some("41"), ..Want::default() } },
            goal_seconds: 60,
        },
        Task {
            title: "Реализация".into(),
            text: format!("АО «Офис-Плюс» купило {q_out} пачек бумаги А4 за {} ₽ с НДС. Оформи реализацию в феврале и проведи.", Money::rub(q_out * price_out)),
            hint: "⬆ Продажи → «+ Реализация товаров». Посмотри проводки: выручка 62/90.01, НДС 90.03/68.02, себестоимость 90.02/41.".into(),
            check: Check::Posted { kind: DocKind::Sale, month: feb, want: Want { party: Some(4), item: Some(1), qty: Some(q_out), amount: Some(Money::rub(q_out * price_out)), ..Want::default() } },
            goal_seconds: 50,
        },
        Task {
            title: "Оплата от покупателя".into(),
            text: "АО «Офис-Плюс» перечислило на расчётный счёт весь свой долг. Оформи поступление на счёт так, чтобы на 28.02 долг покупателя был 0.".into(),
            hint: "Сколько должен покупатель — смотри 📊 Отчёты → Карточка счёта 62 или Справочники → Контрагенты. Затем 💰 Деньги → «+ Поступление на расчётный счёт».".into(),
            check: Check::Balance { account: "62", sub: Some(Sub::Counterparty(4)), at: end, want: Money::ZERO },
            goal_seconds: 70,
        },
        Task {
            title: "Оплата поставщику".into(),
            text: "Погаси весь долг перед ООО «Бумажный двор» с расчётного счёта (с учётом февральской поставки). На 28.02 сальдо 60 по нему должно быть 0.".into(),
            hint: "Долг = кредитовое сальдо 60 по поставщику. 💰 Деньги → «+ Списание с расчётного счёта», вид «Оплата поставщику».".into(),
            check: Check::Balance { account: "60", sub: Some(Sub::Counterparty(1)), at: end, want: Money::ZERO },
            goal_seconds: 70,
        },
        Task {
            title: "Аренда".into(),
            text: "Пришёл акт от ООО «БЦ Северный»: аренда офиса за февраль 61 000 ₽, в т.ч. НДС 22%. Отрази на счёте 26 и проведи.".into(),
            hint: "⬇ Закупки → «+ Поступление услуг», счёт затрат 26, НДС 22%.".into(),
            check: Check::Posted { kind: DocKind::Services, month: feb, want: Want { party: Some(5), amount: Some(Money::rub(61_000)), account: Some("26"), ..Want::default() } },
            goal_seconds: 45,
        },
        Task {
            title: "Под отчёт".into(),
            text: format!("Выдай Ким Анне Викторовне {} ₽ из кассы под отчёт.", Money::rub(advance)),
            hint: "💰 Деньги → «+ Расходный кассовый ордер», вид «Выдача под отчёт». Проводка Дт 71 Кт 50.".into(),
            check: Check::Posted { kind: DocKind::CashOut, month: feb, want: Want { employee: Some(3), amount: Some(Money::rub(advance)), ..Want::default() } },
            goal_seconds: 35,
        },
        Task {
            title: "НДФЛ за январь".into(),
            text: "Перечисли в бюджет НДФЛ, удержанный с январской зарплаты (сальдо 68.01 на 31.01).".into(),
            hint: "Сумма — кредитовое сальдо 68.01 на 31.01 в ОСВ. 💰 Деньги → «+ Списание с расчётного счёта», вид «Уплата налога», налог 68.01.".into(),
            check: Check::Posted { kind: DocKind::BankOut, month: feb, want: Want { amount: Some(Money::rub(35_750)), account: Some("68.01"), ..Want::default() } },
            goal_seconds: 60,
        },
        Task {
            title: "Зарплата за февраль".into(),
            text: "Начисли зарплату за февраль всем трём сотрудникам по окладу на счёт 26.".into(),
            hint: "👥 Сотрудники → «+ Начисление зарплаты»: оклады подставятся сами. Посмотри проводки НДФЛ и взносов.".into(),
            check: Check::Posted { kind: DocKind::Payroll, month: feb, want: Want { amount: Some(Money::rub(275_000)), account: Some("26"), lines: Some(3), ..Want::default() } },
            goal_seconds: 40,
        },
        Task {
            title: "Закрытие февраля".into(),
            text: "Закрой февраль: после закрытия на 28.02 у счетов 19, 26 и 44 не должно быть остатка.".into(),
            hint: "🏁 Конец месяца → «+ Закрытие месяца», дата 28.02.2026.".into(),
            check: Check::Closed { accounts: &["19", "26", "44"], at: end },
            goal_seconds: 30,
        },
        Task {
            title: "НДС к уплате".into(),
            text: "Сколько НДС к уплате на 28.02? Это кредитовое сальдо счёта 68.02. Введи сумму.".into(),
            hint: "📊 Отчёты → Оборотно-сальдовая ведомость за январь–февраль, строка 68.02, «Кон. Кт».".into(),
            check: Check::ReadBalance { account: "68.02", at: end, credit: true },
            goal_seconds: 40,
        },
        Task {
            title: "Прибыль от продаж".into(),
            text: "Какой финансовый результат от продаж за февраль? Убыток — со знаком минус.".into(),
            hint: "📊 Отчёты → Финансовый результат, период февраль.".into(),
            check: Check::ReadSalesProfit { month: feb },
            goal_seconds: 40,
        },
    ]
}
