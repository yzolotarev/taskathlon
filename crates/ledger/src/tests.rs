use crate::catalog::{Sub, Vat};
use crate::document::{self, Body, GoodsLine};
use crate::money::{Date, Money};
use crate::report::{self, account_balance, balance, trial_balance};
use crate::{Book, demo};

fn d(m: u8, day: u8) -> Date {
    Date::new(2026, m, day).unwrap()
}

fn rub(r: i64) -> Money {
    Money::rub(r)
}

#[test]
fn money_parses_rounds_and_prints() {
    assert_eq!(Money::parse("1 234,56"), Some(Money(123_456)));
    assert_eq!(Money::parse("-10.5"), Some(Money(-1_050)));
    assert_eq!(Money::parse("1,234"), None);
    assert_eq!(Money::parse("abc"), None);
    assert_eq!(Money(123_456_789).to_string(), "1 234 567,89");
    assert_eq!(Money(-5).to_string(), "-0,05");
    assert_eq!(Vat::R22.inside(rub(122)), rub(22));
    assert_eq!(Vat::R10.inside(rub(110)), rub(10));
    assert_eq!(Money(100).mul_div(1, 3), Money(33));
    assert_eq!(Money(200).mul_div(1, 3), Money(67));
    assert_eq!(Date::parse("31.01.2026"), Some(d(1, 31)));
    assert_eq!(Date::parse("2026-02-30"), None);
}

#[test]
fn january_books_balance_and_match_hand_calculation() {
    let b = demo::january().unwrap();
    assert!(b.problems.is_empty(), "{:?}", b.problems);
    let end = d(1, 31);
    assert_eq!(balance(&b, "51", end), rub(533_900));
    assert_eq!(balance(&b, "50", end), rub(16_800));
    assert_eq!(balance(&b, "71", end), Money::ZERO);
    assert_eq!(balance(&b, "62", end), rub(21_960));
    assert_eq!(balance(&b, "60", end), -rub(97_600));
    assert_eq!(balance(&b, "70", end), -rub(239_250));
    assert_eq!(balance(&b, "68.02", end), -rub(1_760));
    assert_eq!(balance(&b, "68.01", end), -rub(35_750));
    assert_eq!(balance(&b, "69", end), -rub(82_500));
    assert_eq!(balance(&b, "19", end), Money::ZERO);
    assert_eq!(balance(&b, "26", end), Money::ZERO);
    assert_eq!(balance(&b, "44", end), Money::ZERO);
    assert_eq!(balance(&b, "90", end), Money::ZERO);
    assert_eq!(balance(&b, "91", end), Money::ZERO);
    assert_eq!(balance(&b, "99", end), rub(384_200));
    assert_eq!(balance(&b, "41", end), rub(80_000 + 30_000 - 60_000));
    let r = report::results(&b, d(1, 1), end);
    assert_eq!(r.revenue, rub(107_360));
    assert_eq!(r.vat, rub(19_360));
    assert_eq!(r.cost_of_sales, rub(60_000));
    assert_eq!(r.sales_profit, -rub(382_700));
    assert_eq!(r.tax, Money::ZERO);
    // Stock quantities by item.
    let stock = account_balance(&b, "41", d(1, 1), end);
    let qty: Vec<i64> = stock.iter().map(|s| s.qty).collect();
    assert_eq!(qty, vec![80, 40, 60]);
}

#[test]
fn trial_balance_always_balances() {
    let b = demo::january().unwrap();
    for (from, to) in [(d(1, 1), d(1, 31)), (d(1, 15), d(1, 31)), (d(2, 1), d(2, 28))] {
        let tb = trial_balance(&b, from, to);
        let t = tb.last().unwrap();
        assert_eq!(t.name, "Итого");
        assert_eq!(t.turn_dt, t.turn_kt);
        assert_eq!(t.open_dt - t.open_kt, Money::ZERO, "{from}");
        assert_eq!(t.close_dt - t.close_kt, Money::ZERO, "{from}");
    }
}

#[test]
fn unposting_a_receipt_breaks_a_later_sale_and_posting_short_stock_fails() {
    let mut b = demo::january().unwrap();
    let receipt = b.docs.iter().find(|d| d.body.kind() == crate::DocKind::Receipt).unwrap().id;
    let sale = b.docs.iter().find(|d| d.body.kind() == crate::DocKind::Sale).unwrap().id;
    b.unpost(receipt).unwrap();
    assert_eq!(b.entries_of(receipt).count(), 0);
    assert!(b.problems.iter().any(|(id, m)| *id == sale && m.contains("Не хватает")), "{:?}", b.problems);
    b.post(receipt).unwrap();
    assert!(b.problems.is_empty());
    let id = b.add(d(1, 30), Body::Sale { customer: 3, lines: vec![GoodsLine { item: 1, qty: 1_000, amount: rub(1_000) }] });
    assert!(b.post(id).is_err());
    assert!(!b.doc(id).unwrap().posted);
    assert!(b.problems.is_empty());
    b.remove(id).unwrap();
    assert!(b.remove(receipt).is_err(), "posted documents can't be deleted");
}

#[test]
fn expanded_balance_keeps_debit_and_credit_of_different_suppliers() {
    let mut b = demo::company().unwrap();
    // Overpay one supplier: it now owes us (debit), while another is still owed (credit).
    b.add_posted(d(1, 10), Body::BankOut { to: document::BankOut::Supplier(2), amount: rub(7_000) }).unwrap();
    let tb = trial_balance(&b, d(1, 1), d(1, 31));
    let r60 = tb.iter().find(|r| r.code == "60").unwrap();
    assert_eq!((r60.close_dt, r60.close_kt), (rub(7_000), rub(50_000)));
}

#[test]
fn asset_is_depreciated_from_the_next_month() {
    let mut b = demo::company().unwrap();
    b.add_posted(d(1, 10), Body::AssetPurchase { supplier: 4, asset: 1, amount: rub(122_000), vat: Vat::R22 }).unwrap();
    b.add_posted(d(1, 31), Body::MonthClose).unwrap();
    assert_eq!(balance(&b, "02", d(1, 31)), Money::ZERO);
    assert_eq!(balance(&b, "01", d(1, 31)), rub(100_000));
    b.add_posted(d(2, 28), Body::MonthClose).unwrap();
    assert_eq!(balance(&b, "02", d(2, 28)), -Money(277_778));
    let (open, lines) = report::account_card(&b, "02", Some(Sub::Asset(1)), d(2, 1), d(2, 28));
    assert_eq!(open, Money::ZERO);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].corr, "26");
}

#[test]
fn profitable_month_accrues_profit_tax() {
    let mut b = demo::company().unwrap();
    b.add_posted(d(1, 5), Body::Sale { customer: 3, lines: vec![GoodsLine { item: 1, qty: 100, amount: rub(122_000) }] }).unwrap();
    b.add_posted(d(1, 31), Body::MonthClose).unwrap();
    // Revenue 122 000 − VAT 22 000 − cost 30 000 = 70 000 profit, tax 25%.
    let r = report::results(&b, d(1, 1), d(1, 31));
    assert_eq!(r.sales_profit, rub(70_000));
    assert_eq!(r.tax, rub(17_500));
    assert_eq!(r.net, rub(52_500));
}

#[test]
fn bad_input_is_an_error_not_a_crash() {
    let mut b = Book::new("Тест");
    assert!(b.add_posted(d(1, 1), Body::Receipt { supplier: 99, account: "41".into(), lines: vec![] }).is_err());
    assert!(b.add_posted(d(1, 1), Body::Payroll { account: "68".into(), lines: vec![] }).is_ok(), "no lines, nothing to post");
    let id = b.add(d(1, 1), Body::BankOut { to: document::BankOut::Tax("51".into()), amount: rub(1) });
    assert!(b.post(id).is_err());
    assert!(b.post(9_999).is_err());
    assert!(b.entries.is_empty());
}

#[test]
fn json_round_trip_reposts() {
    let b = demo::january().unwrap();
    let s = b.to_json().unwrap();
    let c = Book::from_json(&s).unwrap();
    assert_eq!(c.entries, b.entries);
}

#[test]
fn february_workshop_is_solvable_and_checks_are_strict() {
    use crate::practice::{Check, february};
    for seed in [1, 7, 42, 2026] {
        let tasks = february(seed);
        let mut b = demo::january().unwrap();
        let end = d(2, 28);
        for t in &tasks {
            assert!(t.check(&b, None).is_err() || t.asks_number(), "{} passes before doing anything", t.title);
        }
        for t in &tasks {
            match &t.check {
                Check::Posted { kind, want, .. } => {
                    let amount = want.amount.unwrap_or_default();
                    let body = match kind {
                        crate::DocKind::Receipt => {
                            Body::Receipt { supplier: 1, account: "41".into(), lines: vec![GoodsLine { item: 1, qty: want.qty.unwrap(), amount }] }
                        }
                        crate::DocKind::Sale => Body::Sale { customer: 4, lines: vec![GoodsLine { item: 1, qty: want.qty.unwrap(), amount }] },
                        crate::DocKind::Services => Body::Services {
                            supplier: 5,
                            lines: vec![document::ExpenseLine { text: "Аренда".into(), amount, vat: Vat::R22, account: "26".into() }],
                        },
                        crate::DocKind::CashOut => Body::CashOut { to: document::CashOut::Advance(3), amount },
                        crate::DocKind::BankOut => Body::BankOut { to: document::BankOut::Tax("68.01".into()), amount },
                        crate::DocKind::Payroll => {
                            Body::Payroll { account: "26".into(), lines: vec![(1, rub(120_000)), (2, rub(80_000)), (3, rub(75_000))] }
                        }
                        _ => continue,
                    };
                    b.add_posted(d(2, 10), body).unwrap();
                }
                Check::Balance { account, sub: Some(Sub::Counterparty(c)), .. } => {
                    let owed = report::account_balance(&b, account, end, end)
                        .iter()
                        .find(|r| r.sub == Some(Sub::Counterparty(*c)))
                        .map(|r| r.row.close_dt - r.row.close_kt)
                        .unwrap();
                    let body = if *account == "62" {
                        Body::BankIn { from: document::BankIn::Customer(*c), amount: owed }
                    } else {
                        Body::BankOut { to: document::BankOut::Supplier(*c), amount: -owed }
                    };
                    b.add_posted(d(2, 20), body).unwrap();
                }
                Check::Closed { .. } => {
                    b.add_posted(end, Body::MonthClose).unwrap();
                }
                _ => {}
            }
        }
        for t in &tasks {
            let answer = t.expected(&b);
            assert_eq!(t.check(&b, answer), Ok(()), "seed {seed}: {}", t.title);
            if t.asks_number() {
                assert!(t.check(&b, answer.map(|a| a + Money(1))).is_err());
            }
        }
    }
}
