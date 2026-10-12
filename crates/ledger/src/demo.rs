//! A small demo company for practice: a wholesale trader of office supplies with a quarter
//! of typical documents. All names are invented.

use crate::catalog::{Sub, Vat};
use crate::document::{self, Body, ExpenseLine, GoodsLine, OpeningLine};
use crate::money::{Date, Money};
use crate::{Book, LedgerError};

fn d(m: u8, day: u8) -> Date {
    Date::new(2026, m, day).unwrap_or(Date { y: 2026, m: 1, d: 1 })
}

/// An empty company with catalogs filled in and opening balances posted.
pub fn company() -> Result<Book, LedgerError> {
    let mut b = Book::new("ООО «Ромашка-Опт»");
    let c = &mut b.catalogs;
    let paper = c.add_counterparty("ООО «Бумажный двор»", "7701000001");
    let _furn = c.add_counterparty("ИП Соколов А. В.", "500100000002");
    let _shop = c.add_counterparty("ООО «Канцмир»", "7702000003");
    let _office = c.add_counterparty("АО «Офис-Плюс»", "7703000004");
    let _landlord = c.add_counterparty("ООО «БЦ Северный»", "7704000005");
    let _bank = c.add_counterparty("Банк «Пример»", "7705000006");
    let a4 = c.add_item("Бумага А4, пачка", "шт", Vat::R22);
    let pens = c.add_item("Ручки шариковые, упаковка", "шт", Vat::R22);
    let _folders = c.add_item("Папки-регистраторы", "шт", Vat::R22);
    let _cart = c.add_item("Картридж для принтера", "шт", Vat::R22);
    let _book = c.add_item("Учебник по бухучёту", "шт", Vat::R10);
    let _boxes = c.add_item("Коробки упаковочные (материал)", "шт", Vat::R22);
    let _ivanova = c.add_employee("Иванова Мария Сергеевна", "Директор", Money::rub(120_000));
    let _petrov = c.add_employee("Петров Олег Иванович", "Менеджер по продажам", Money::rub(80_000));
    let _kim = c.add_employee("Ким Анна Викторовна", "Бухгалтер", Money::rub(75_000));
    let _laptop = c.add_asset("Ноутбук бухгалтера", 36);
    b.add_posted(
        d(1, 1),
        Body::Opening {
            lines: vec![
                OpeningLine { account: "51".into(), sub: None, debit: Money::rub(500_000), credit: Money::ZERO, qty: 0 },
                OpeningLine { account: "50".into(), sub: None, debit: Money::rub(20_000), credit: Money::ZERO, qty: 0 },
                OpeningLine { account: "41".into(), sub: Some(Sub::Item(a4)), debit: Money::rub(60_000), credit: Money::ZERO, qty: 200 },
                OpeningLine { account: "41".into(), sub: Some(Sub::Item(pens)), debit: Money::rub(20_000), credit: Money::ZERO, qty: 100 },
                OpeningLine { account: "60".into(), sub: Some(Sub::Counterparty(paper)), debit: Money::ZERO, credit: Money::rub(50_000), qty: 0 },
                OpeningLine { account: "80".into(), sub: None, debit: Money::ZERO, credit: Money::rub(10_000), qty: 0 },
                OpeningLine { account: "84".into(), sub: None, debit: Money::ZERO, credit: Money::rub(540_000), qty: 0 },
            ],
        },
    )?;
    Ok(b)
}

/// The demo company with a month of typical January operations posted.
pub fn january() -> Result<Book, LedgerError> {
    let mut b = company()?;
    let (paper, shop, office, landlord) = (1, 3, 4, 5);
    let (a4, pens, folders) = (1, 2, 3);
    let (ivanova, petrov, kim) = (1, 2, 3);
    let docs = vec![
        (
            d(1, 12),
            Body::Receipt { supplier: paper, account: "41".into(), lines: vec![GoodsLine { item: folders, qty: 100, amount: Money::rub(36_600) }] },
        ),
        (
            d(1, 14),
            Body::Sale {
                customer: shop,
                lines: vec![
                    GoodsLine { item: a4, qty: 120, amount: Money::rub(61_000) },
                    GoodsLine { item: folders, qty: 40, amount: Money::rub(24_400) },
                ],
            },
        ),
        (d(1, 16), Body::BankOut { to: document::BankOut::Supplier(paper), amount: Money::rub(50_000) }),
        (d(1, 19), Body::BankIn { from: document::BankIn::Customer(shop), amount: Money::rub(85_400) }),
        (
            d(1, 20),
            Body::Services {
                supplier: landlord,
                lines: vec![ExpenseLine {
                    text: "Аренда офиса за январь".into(), amount: Money::rub(61_000), vat: Vat::R22, account: "26".into()
                }],
            },
        ),
        (d(1, 21), Body::CashOut { to: document::CashOut::Advance(petrov), amount: Money::rub(5_000) }),
        (
            d(1, 23),
            Body::AdvanceReport {
                employee: petrov,
                lines: vec![ExpenseLine {
                    text: "Такси на встречу с клиентом".into(),
                    amount: Money::rub(3_200),
                    vat: Vat::None,
                    account: "44".into(),
                }],
            },
        ),
        (d(1, 24), Body::CashIn { from: document::CashIn::AdvanceReturn(petrov), amount: Money::rub(1_800) }),
        (d(1, 26), Body::Sale { customer: office, lines: vec![GoodsLine { item: pens, qty: 60, amount: Money::rub(21_960) }] }),
        (d(1, 28), Body::BankOut { to: document::BankOut::Fee, amount: Money::rub(1_500) }),
        (
            d(1, 31),
            Body::Payroll {
                account: "26".into(),
                lines: vec![(ivanova, Money::rub(120_000)), (petrov, Money::rub(80_000)), (kim, Money::rub(75_000))],
            },
        ),
        (d(1, 31), Body::MonthClose),
    ];
    for (date, body) in docs {
        b.add_posted(date, body)?;
    }
    Ok(b)
}
