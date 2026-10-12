//! The chart of accounts: a working subset of the Russian chart for commercial organisations
//! (Order of the Ministry of Finance No. 94n, a public legal act). Subaccount numbers follow the
//! convention accountants use in practice.

use serde::{Deserialize, Serialize};

/// Whether the balance may only be debit, only credit, or either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Active,
    Passive,
    ActivePassive,
}

/// What a balance on the account is broken down by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Analytic {
    None,
    Counterparty,
    Item,
    Employee,
    Asset,
}

#[derive(Clone, Copy, Debug)]
pub struct Account {
    pub code: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    pub analytic: Analytic,
    /// Quantities are tracked alongside amounts (stock).
    pub quantity: bool,
    /// Has subaccounts; entries go to those, never to this one.
    pub group: bool,
}

const fn acc(code: &'static str, name: &'static str, kind: Kind, analytic: Analytic) -> Account {
    Account { code, name, kind, analytic, quantity: false, group: false }
}

const fn stock(code: &'static str, name: &'static str) -> Account {
    Account { code, name, kind: Kind::Active, analytic: Analytic::Item, quantity: true, group: false }
}

const fn group(code: &'static str, name: &'static str, kind: Kind) -> Account {
    Account { code, name, kind, analytic: Analytic::None, quantity: false, group: true }
}

use Analytic as A;
use Kind::*;

pub const CHART: &[Account] = &[
    acc("01", "Основные средства", Active, A::Asset),
    acc("02", "Амортизация основных средств", Passive, A::Asset),
    acc("08", "Вложения во внеоборотные активы", Active, A::Asset),
    stock("10", "Материалы"),
    acc("19", "НДС по приобретённым ценностям", Active, A::Counterparty),
    acc("20", "Основное производство", Active, A::None),
    acc("26", "Общехозяйственные расходы", Active, A::None),
    stock("41", "Товары"),
    acc("44", "Расходы на продажу", Active, A::None),
    acc("50", "Касса", Active, A::None),
    acc("51", "Расчётные счета", Active, A::None),
    acc("60", "Расчёты с поставщиками и подрядчиками", ActivePassive, A::Counterparty),
    acc("62", "Расчёты с покупателями и заказчиками", ActivePassive, A::Counterparty),
    acc("66", "Расчёты по краткосрочным кредитам и займам", Passive, A::Counterparty),
    group("68", "Расчёты по налогам и сборам", ActivePassive),
    acc("68.01", "НДФЛ", ActivePassive, A::None),
    acc("68.02", "НДС", ActivePassive, A::None),
    acc("68.04", "Налог на прибыль", ActivePassive, A::None),
    acc("69", "Расчёты по социальному страхованию", ActivePassive, A::None),
    acc("70", "Расчёты с персоналом по оплате труда", Passive, A::Employee),
    acc("71", "Расчёты с подотчётными лицами", ActivePassive, A::Employee),
    acc("76", "Расчёты с разными дебиторами и кредиторами", ActivePassive, A::Counterparty),
    acc("80", "Уставный капитал", Passive, A::None),
    acc("84", "Нераспределённая прибыль (непокрытый убыток)", ActivePassive, A::None),
    group("90", "Продажи", ActivePassive),
    acc("90.01", "Выручка", Passive, A::None),
    acc("90.02", "Себестоимость продаж", Active, A::None),
    acc("90.03", "НДС", Active, A::None),
    acc("90.07", "Расходы на продажу", Active, A::None),
    acc("90.08", "Управленческие расходы", Active, A::None),
    acc("90.09", "Прибыль / убыток от продаж", ActivePassive, A::None),
    group("91", "Прочие доходы и расходы", ActivePassive),
    acc("91.01", "Прочие доходы", Passive, A::None),
    acc("91.02", "Прочие расходы", Active, A::None),
    acc("91.09", "Сальдо прочих доходов и расходов", ActivePassive, A::None),
    acc("99", "Прибыли и убытки", ActivePassive, A::None),
    // Off-balance counter account for opening balances.
    acc("000", "Вспомогательный (ввод остатков)", ActivePassive, A::None),
];

pub fn account(code: &str) -> Option<&'static Account> {
    CHART.iter().find(|a| a.code == code)
}

/// The account itself and its group parent ("68.02" → ["68.02", "68"]).
pub fn rolls_into(code: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    if let Some(a) = account(code) {
        out.push(a.code);
        if let Some((parent, _)) = code.split_once('.')
            && let Some(p) = account(parent)
        {
            out.push(p.code);
        }
    }
    out
}

/// Accounts that can carry entries (no groups).
pub fn postable() -> impl Iterator<Item = &'static Account> {
    CHART.iter().filter(|a| !a.group)
}
