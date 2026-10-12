//! Accounting basics for an assistant accountant: chart of accounts, entries, VAT, payroll.
//! Simplified Russian practice: rates are fixed training values, not tax advice.

use super::{Gen, PEOPLE};
use crate::check::Kind;
use crate::sheet::{Cell, a1};
use crate::{Result, Task};

pub const ACCOUNTS: &[(&str, f64)] = &[
    ("Касса", 50.0),
    ("Расчётный счёт", 51.0),
    ("Расчёты с поставщиками", 60.0),
    ("Расчёты с покупателями", 62.0),
    ("Расчёты с персоналом по оплате труда", 70.0),
    ("Расчёты по налогам и сборам", 68.0),
    ("Расчёты по страховым взносам", 69.0),
    ("Товары", 41.0),
    ("Материалы", 10.0),
    ("Основные средства", 1.0),
    ("Амортизация основных средств", 2.0),
    ("НДС по приобретённым ценностям", 19.0),
    ("Расчёты с подотчётными лицами", 71.0),
    ("Продажи", 90.0),
    ("Прочие доходы и расходы", 91.0),
    ("Прибыли и убытки", 99.0),
    ("Уставный капитал", 80.0),
    ("Нераспределённая прибыль", 84.0),
    ("Общехозяйственные расходы", 26.0),
    ("Расходы на продажу", 44.0),
];

/// Operation, debit, credit.
pub const ENTRIES: &[(&str, f64, f64)] = &[
    ("Оплатили поставщику с расчётного счёта", 60.0, 51.0),
    ("Покупатель оплатил на расчётный счёт", 51.0, 62.0),
    ("Получили товары от поставщика", 41.0, 60.0),
    ("Выделили входящий НДС по товарам", 19.0, 60.0),
    ("Отразили выручку от продажи", 62.0, 90.0),
    ("Начислили НДС с выручки", 90.0, 68.0),
    ("Списали себестоимость проданных товаров", 90.0, 41.0),
    ("Начислили зарплату офису", 26.0, 70.0),
    ("Удержали НДФЛ из зарплаты", 70.0, 68.0),
    ("Выплатили зарплату с расчётного счёта", 70.0, 51.0),
    ("Начислили страховые взносы с зарплаты офиса", 26.0, 69.0),
    ("Перечислили НДФЛ в бюджет", 68.0, 51.0),
    ("Сняли наличные с расчётного счёта в кассу", 50.0, 51.0),
    ("Выдали деньги под отчёт из кассы", 71.0, 50.0),
    ("Начислили амортизацию офисного компьютера", 26.0, 2.0),
    ("Приняли входящий НДС к вычету", 68.0, 19.0),
];

pub fn accounts(g: &mut Gen) -> Result<Task> {
    let n = match g.level {
        1 => 3,
        2 => 5,
        3 => 7,
        _ => 10,
    };
    let t = g.table("План счетов", &["Что учитываем", "Счёт"], n);
    g.setup.width(t.col(0), 300.0);
    let mut idx: Vec<usize> = (0..ACCOUNTS.len()).collect();
    if g.level <= 2 {
        idx.truncate(8);
    }
    g.l.shuffle(&mut idx);
    let mut cheat = Vec::new();
    let mut first = None;
    for (i, r) in t.rows().enumerate() {
        let Some(&(name, no)) = idx.get(i).and_then(|k| ACCOUNTS.get(*k)) else { continue };
        g.setup.text(t.cell(0, r), name);
        let cell = t.cell(1, r);
        g.setup.answer(cell.clone());
        g.check(&cell, Kind::Value { cell: cell.clone(), expected: Cell::Num(no) });
        cheat.push(format!("{name} — {no:02}"));
        first.get_or_insert(cell);
    }
    let prompt = match g.level {
        1 => format!("Впиши номера счетов. Шпаргалка: {}.", cheat.join("; ")),
        _ => "Впиши номер счёта из плана счетов для каждой строки.".into(),
    };
    let mut task = g.task("План счетов", prompt, None, first.unwrap_or_else(|| "B2".into()));
    task.hints = vec![
        "Деньги: 50 касса, 51 расчётный счёт. Расчёты: 60 поставщики, 62 покупатели, 70 зарплата, 68 налоги, 69 взносы, 71 подотчёт.".into(),
        "Имущество: 01 ОС, 02 амортизация, 10 материалы, 41 товары, 19 входящий НДС. Итоги: 90 продажи, 91 прочие, 99 прибыль. Затраты: 26, 44."
            .into(),
        cheat.join("; "),
    ];
    Ok(task)
}

pub fn entries(g: &mut Gen) -> Result<Task> {
    let n = match g.level {
        1 => 2,
        2 => 4,
        3 => 6,
        _ => 8,
    };
    let t = g.table("Журнал операций", &["Операция", "Дт", "Кт"], n);
    g.setup.width(t.col(0), 360.0);
    let mut idx: Vec<usize> = (0..ENTRIES.len()).collect();
    if g.level <= 2 {
        idx.truncate(8);
    }
    g.l.shuffle(&mut idx);
    let mut solution = Vec::new();
    let mut first = None;
    for (i, r) in t.rows().enumerate() {
        let Some(&(op, dt, kt)) = idx.get(i).and_then(|k| ENTRIES.get(*k)) else { continue };
        g.setup.text(t.cell(0, r), op);
        for (c, v) in [(1, dt), (2, kt)] {
            let cell = t.cell(c, r);
            g.setup.answer(cell.clone());
            g.check(&cell, Kind::Value { cell: cell.clone(), expected: Cell::Num(v) });
        }
        solution.push(format!("{op}: Дт {dt:02} Кт {kt:02}"));
        first.get_or_insert(t.cell(1, r));
    }
    let prompt = match g.level {
        1 => "Составь проводки: в «Дт» — счёт, куда пришло, в «Кт» — откуда ушло. Пример: оплата поставщику с расчётного счёта = Дт 60 Кт 51."
            .to_string(),
        _ => "Составь проводки для операций.".to_string(),
    };
    let mut task = g.task("Проводки", prompt, None, first.unwrap_or_else(|| "B2".into()));
    task.hints = vec![
        "Дебет — что увеличилось у нас (деньги на 51, товар на 41) или долг уменьшился. Кредит — источник.".into(),
        "Начисление зарплаты: Дт 26 Кт 70. НДФЛ: Дт 70 Кт 68. Выплата: Дт 70 Кт 51. Выручка: Дт 62 Кт 90.".into(),
        solution.join("; "),
    ];
    task.goal_seconds = 25.0 * n as f32;
    Ok(task)
}

pub fn vat_extract(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Входящие счета", &["Поставщик", "Сумма с НДС"], n);
    for r in t.rows() {
        let p = g.d.pick(super::PARTIES);
        g.txt(&t, 0, r, p);
        let v = g.d.amount(1_200, 600_000, 12);
        g.num(&t, 1, r, v);
    }
    let b = t.col(1);
    Ok(g.column(
        "Выделить НДС",
        &t,
        2,
        "в т.ч. НДС 20%",
        |r| format!("=ROUND({}*20/120,2)", a1(r, b)),
        "Выдели НДС из суммы: ОКРУГЛ(сумма*20/120; 2).",
        "Сколько НДС 20% внутри каждого счёта? До копеек.",
    ))
}

pub fn payroll(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Зарплата за месяц", &["Сотрудник", "Начислено"], n);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, PEOPLE.get(i % PEOPLE.len()).copied().unwrap_or("Сотрудник"));
        let v = g.d.amount(30_000, 180_000, 100);
        g.num(&t, 1, r, v);
    }
    let b = t.col(1);
    if g.level <= 2 {
        return Ok(g.column(
            "НДФЛ",
            &t,
            2,
            "НДФЛ 13%",
            |r| format!("=ROUND({}*0.13,0)", a1(r, b)),
            "НДФЛ = ОКРУГЛ(начислено*13%; 0) — до рубля.",
            "Посчитай НДФЛ 13% по каждому, до рубля.",
        ));
    }
    for (col, head) in [(2, "НДФЛ 13%"), (3, "К выплате")] {
        g.setup.text(t.cell(col, t.header), head);
        g.setup.bold.push(t.cell(col, t.header));
        g.setup.width(t.col(col), 120.0);
    }
    for r in t.rows() {
        for col in [2, 3] {
            let cell = t.cell(col, r);
            g.setup.answer(cell.clone());
            let f = if col == 2 { format!("=ROUND({}*0.13,0)", a1(r, b)) } else { format!("={0}-ROUND({0}*0.13,0)", a1(r, b)) };
            g.formula_check(&cell, &f);
        }
    }
    let first = t.cell(2, t.first);
    let prompt = "Заполни НДФЛ 13% (до рубля) и сумму к выплате (начислено − НДФЛ).".to_string();
    let mut task = g.task("НДФЛ и к выплате", prompt, None, first);
    task.goal_seconds = 90.0;
    Ok(task)
}

pub fn insurance(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Взносы за месяц", &["Сотрудник", "Начислено"], n);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, PEOPLE.get(i % PEOPLE.len()).copied().unwrap_or("Сотрудник"));
        let v = g.d.amount(30_000, 180_000, 100);
        g.num(&t, 1, r, v);
    }
    let b = t.col(1);
    let mut task = g.column(
        "Страховые взносы",
        &t,
        2,
        "Взносы 30%",
        |r| format!("=ROUND({}*0.3,2)", a1(r, b)),
        "Взносы = ОКРУГЛ(начислено*30%; 2).",
        "Посчитай страховые взносы работодателя по ставке 30% (учебная ставка) до копеек.",
    );
    if g.level >= 3 {
        let total = t.cell(2, t.last + 1);
        task.setup.text(t.cell(0, t.last + 1), "Итого");
        task.setup.answer(total.clone());
        task.checks.push(crate::check::Check {
            label: total.clone(),
            skill: "acc.insurance".into(),
            kind: Kind::Formula {
                cell: total,
                reference: format!("={}", t.rows().map(|r| format!("ROUND({}*0.3,2)", a1(r, b))).collect::<Vec<_>>().join("+")),
                live: true,
            },
        });
        task.prompt.push_str(" И итог под таблицей.");
    }
    Ok(task)
}

pub fn osv(g: &mut Gen) -> Result<Task> {
    let n = g.rows().min(8);
    let t = g.table("Оборотно-сальдовая ведомость", &["Счёт", "Сальдо нач.", "Оборот Дт", "Оборот Кт"], n);
    let active = [
        ("01", "Основные средства"),
        ("10", "Материалы"),
        ("41", "Товары"),
        ("50", "Касса"),
        ("51", "Расчётный счёт"),
        ("62", "Покупатели"),
        ("71", "Подотчёт"),
        ("19", "НДС входящий"),
    ];
    let mut order: Vec<usize> = (0..active.len()).collect();
    g.l.shuffle(&mut order);
    for (i, r) in t.rows().enumerate() {
        let (no, name) = order.get(i).and_then(|k| active.get(*k)).copied().unwrap_or(("51", "Расчётный счёт"));
        g.txt(&t, 0, r, &format!("{no} {name}"));
        let start = g.d.amount(0, 500_000, 100);
        let dt = g.d.amount(0, 300_000, 100);
        let kt = g.d.amount(0, (start + dt) as i64, 100);
        g.num(&t, 1, r, start);
        g.num(&t, 2, r, dt);
        g.num(&t, 3, r, kt);
    }
    g.setup.width(t.col(0), 180.0);
    let (b, c, d) = (t.col(1), t.col(2), t.col(3));
    Ok(g.column(
        "Сальдо в ОСВ",
        &t,
        4,
        "Сальдо кон.",
        |r| format!("={}+{}-{}", a1(r, b), a1(r, c), a1(r, d)),
        "Все счета активные: сальдо на конец = начальное + оборот Дт − оборот Кт.",
        "Досчитай конечное сальдо по каждому (активному) счёту.",
    ))
}
