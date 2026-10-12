//! Documents: the only way anything gets into the books. Posting a document turns it into
//! double entries; unposting removes them.

use crate::catalog::{Sub, Vat};
use crate::money::{Date, Money};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsLine {
    pub item: u32,
    pub qty: i64,
    /// Line total including VAT.
    pub amount: Money,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpenseLine {
    pub text: String,
    /// Line total including VAT.
    pub amount: Money,
    pub vat: Vat,
    /// Cost account: "26", "44", "20" or "10".
    pub account: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpeningLine {
    pub account: String,
    pub sub: Option<Sub>,
    pub debit: Money,
    pub credit: Money,
    pub qty: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BankIn {
    /// Payment from a customer (62).
    Customer(u32),
    /// A loan received (66).
    Loan(u32),
    /// Cash deposited from the till (50).
    FromCash,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BankOut {
    Supplier(u32),
    /// Tax paid from the given account ("68.01", "68.02", "68.04").
    Tax(String),
    Insurance,
    Salary(u32),
    /// Bank fee (91.02).
    Fee,
    LoanRepay(u32),
    /// Cash withdrawn into the till (50).
    ToCash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CashIn {
    Customer(u32),
    /// An employee returns an unspent advance.
    AdvanceReturn(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CashOut {
    Advance(u32),
    Salary(u32),
    Supplier(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Body {
    Opening {
        lines: Vec<OpeningLine>,
    },
    /// Goods or materials from a supplier into 41 or 10.
    Receipt {
        supplier: u32,
        account: String,
        lines: Vec<GoodsLine>,
    },
    /// Services or other costs from a supplier.
    Services {
        supplier: u32,
        lines: Vec<ExpenseLine>,
    },
    /// Goods shipped to a customer from 41.
    Sale {
        customer: u32,
        lines: Vec<GoodsLine>,
    },
    /// A fixed asset bought and put into service.
    AssetPurchase {
        supplier: u32,
        asset: u32,
        amount: Money,
        vat: Vat,
    },
    /// Materials written off to a cost account.
    WriteOff {
        account: String,
        lines: Vec<GoodsLine>,
    },
    BankIn {
        from: BankIn,
        amount: Money,
    },
    BankOut {
        to: BankOut,
        amount: Money,
    },
    CashIn {
        from: CashIn,
        amount: Money,
    },
    CashOut {
        to: CashOut,
        amount: Money,
    },
    AdvanceReport {
        employee: u32,
        lines: Vec<ExpenseLine>,
    },
    Payroll {
        account: String,
        lines: Vec<(u32, Money)>,
    },
    /// Depreciation, VAT deduction, closing cost accounts, financial result for the month.
    MonthClose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocKind {
    Opening,
    Receipt,
    Services,
    Sale,
    AssetPurchase,
    WriteOff,
    BankIn,
    BankOut,
    CashIn,
    CashOut,
    AdvanceReport,
    Payroll,
    MonthClose,
}

impl DocKind {
    pub const ALL: [DocKind; 13] = [
        DocKind::Opening,
        DocKind::Receipt,
        DocKind::Services,
        DocKind::Sale,
        DocKind::AssetPurchase,
        DocKind::WriteOff,
        DocKind::BankIn,
        DocKind::BankOut,
        DocKind::CashIn,
        DocKind::CashOut,
        DocKind::AdvanceReport,
        DocKind::Payroll,
        DocKind::MonthClose,
    ];

    pub fn title(self) -> &'static str {
        match self {
            DocKind::Opening => "Ввод начальных остатков",
            DocKind::Receipt => "Поступление товаров и материалов",
            DocKind::Services => "Поступление услуг",
            DocKind::Sale => "Реализация товаров",
            DocKind::AssetPurchase => "Покупка основного средства",
            DocKind::WriteOff => "Списание материалов",
            DocKind::BankIn => "Поступление на расчётный счёт",
            DocKind::BankOut => "Списание с расчётного счёта",
            DocKind::CashIn => "Приходный кассовый ордер",
            DocKind::CashOut => "Расходный кассовый ордер",
            DocKind::AdvanceReport => "Авансовый отчёт",
            DocKind::Payroll => "Начисление зарплаты",
            DocKind::MonthClose => "Закрытие месяца",
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            DocKind::Opening => "ВО",
            DocKind::Receipt => "ПТ",
            DocKind::Services => "ПУ",
            DocKind::Sale => "РТ",
            DocKind::AssetPurchase => "ОС",
            DocKind::WriteOff => "СМ",
            DocKind::BankIn => "ПС",
            DocKind::BankOut => "СС",
            DocKind::CashIn => "ПКО",
            DocKind::CashOut => "РКО",
            DocKind::AdvanceReport => "АО",
            DocKind::Payroll => "ЗП",
            DocKind::MonthClose => "ЗМ",
        }
    }

    /// Same-day order: opening first, month close last.
    pub fn order(self) -> u8 {
        match self {
            DocKind::Opening => 0,
            DocKind::MonthClose => 9,
            _ => 5,
        }
    }
}

impl Body {
    pub fn kind(&self) -> DocKind {
        match self {
            Body::Opening { .. } => DocKind::Opening,
            Body::Receipt { .. } => DocKind::Receipt,
            Body::Services { .. } => DocKind::Services,
            Body::Sale { .. } => DocKind::Sale,
            Body::AssetPurchase { .. } => DocKind::AssetPurchase,
            Body::WriteOff { .. } => DocKind::WriteOff,
            Body::BankIn { .. } => DocKind::BankIn,
            Body::BankOut { .. } => DocKind::BankOut,
            Body::CashIn { .. } => DocKind::CashIn,
            Body::CashOut { .. } => DocKind::CashOut,
            Body::AdvanceReport { .. } => DocKind::AdvanceReport,
            Body::Payroll { .. } => DocKind::Payroll,
            Body::MonthClose => DocKind::MonthClose,
        }
    }

    /// The document's total, as shown in journals.
    pub fn total(&self) -> Money {
        match self {
            Body::Opening { lines } => lines.iter().map(|l| l.debit).sum(),
            Body::Receipt { lines, .. } | Body::Sale { lines, .. } | Body::WriteOff { lines, .. } => lines.iter().map(|l| l.amount).sum(),
            Body::Services { lines, .. } | Body::AdvanceReport { lines, .. } => lines.iter().map(|l| l.amount).sum(),
            Body::AssetPurchase { amount, .. }
            | Body::BankIn { amount, .. }
            | Body::BankOut { amount, .. }
            | Body::CashIn { amount, .. }
            | Body::CashOut { amount, .. } => *amount,
            Body::Payroll { lines, .. } => lines.iter().map(|l| l.1).sum(),
            Body::MonthClose => Money::ZERO,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: u32,
    pub number: String,
    pub date: Date,
    pub posted: bool,
    pub body: Body,
    #[serde(default)]
    pub comment: String,
}

impl Document {
    pub fn label(&self) -> String {
        format!("{} {} от {}", self.body.kind().title(), self.number, self.date)
    }
}
