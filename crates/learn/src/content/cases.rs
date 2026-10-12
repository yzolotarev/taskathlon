//! Open work cases: a message from a manager, raw data, and results graded by value.
//! The method is free; formulas are rewarded where the brief asks for a live table.

use super::{CITIES, DEPTS, Gen, PARTIES, PEOPLE, PRODUCTS};
use crate::check::{Check, Kind};
use crate::sheet::{Cell, a1, date, date_text, num_text};
use crate::{Brief, Mode, Result, Task};
use gridcraft_tr::{t as tr, ts};
use std::collections::BTreeMap;

pub struct CaseDef {
    pub id: &'static str,
    pub title: &'static str,
    pub role: &'static str,
    pub minutes: u32,
    /// Skills that make the case fair; it unlocks when they reach level 2.
    pub requires: &'static [&'static str],
    pub build: fn(&mut Gen) -> Result<Task>,
}

pub const CASES: &[CaseDef] = &[
    CaseDef {
        id: "case.sales",
        title: "Отчёт по продажам",
        role: "Офис-аналитик",
        minutes: 20,
        requires: &["fn.sum", "cond.sumif", "fn.minmax"],
        build: sales,
    },
    CaseDef {
        id: "case.payroll",
        title: "Зарплатная ведомость",
        role: "Помощник бухгалтера",
        minutes: 20,
        requires: &["refs.abs", "fn.round", "acc.payroll"],
        build: payroll,
    },
    CaseDef {
        id: "case.bank",
        title: "Сверка с банковской выпиской",
        role: "Помощник бухгалтера",
        minutes: 20,
        requires: &["cond.countif", "lookup.xlookup", "logic.if"],
        build: bank,
    },
    CaseDef {
        id: "case.act", title: "Акт сверки с контрагентом", role: "Бухгалтер", minutes: 20, requires: &["fn.sum", "acc.entries"], build: act
    },
    CaseDef {
        id: "case.ar",
        title: "Просроченная дебиторка",
        role: "Бухгалтер / аналитик",
        minutes: 25,
        requires: &["date.diff", "logic.if", "cond.sumifs"],
        build: receivables,
    },
    CaseDef {
        id: "case.cleanup",
        title: "Выгрузка из 1С",
        role: "Помощник бухгалтера",
        minutes: 15,
        requires: &["text.trim", "text.number", "cond.sumif"],
        build: cleanup,
    },
    CaseDef {
        id: "case.purchase",
        title: "Сравнение поставщиков",
        role: "Стажёр в отдел закупок",
        minutes: 20,
        requires: &["fn.minmax", "lookup.index", "refs.fill"],
        build: purchase,
    },
    CaseDef {
        id: "case.stock",
        title: "Остатки на складе",
        role: "Офис-аналитик / склад",
        minutes: 15,
        requires: &["start.sub", "logic.if", "start.mul"],
        build: stock,
    },
    CaseDef {
        id: "case.budget",
        title: "План-факт по бюджету",
        role: "Помощник экономиста",
        minutes: 15,
        requires: &["start.sub", "logic.if", "fn.sum"],
        build: budget,
    },
    CaseDef {
        id: "case.vacation",
        title: "График отпусков",
        role: "Стажёр HR / документооборот",
        minutes: 15,
        requires: &["date.diff", "cond.sumif", "fn.minmax"],
        build: vacation,
    },
    CaseDef {
        id: "case.retail",
        title: "Продажи магазинов",
        role: "Аналитик в ритейле",
        minutes: 20,
        requires: &["fn.round", "cond.sumif", "fn.minmax"],
        build: retail,
    },
    CaseDef {
        id: "case.recon",
        title: "Расхождения в двух выгрузках",
        role: "Бухгалтер / аналитик",
        minutes: 30,
        requires: &["lookup.xlookup", "cond.sumif", "text.number"],
        build: recon,
    },
];

pub fn case(id: &str) -> Option<&'static CaseDef> {
    CASES.iter().find(|c| c.id == id)
}

fn push(g: &mut Gen, skill: &str, label: &str, kind: Kind) {
    g.checks.push(Check { label: label.into(), skill: skill.into(), kind });
}

fn constant(v: f64) -> String {
    format!("={v}")
}

fn finish(g: &mut Gen, id: &str, brief: Brief, deliverables: &[String], focus: String) -> Task {
    let def = case(id);
    let mut t = g.task(def.map(|d| ts(d.title)).unwrap_or(ts("Кейс")), String::new(), None, focus);
    t.spec.skill = id.into();
    t.mode = Mode::Case;
    t.prompt = deliverables.iter().enumerate().map(|(i, d)| format!("{}. {d}", i + 1)).collect::<Vec<_>>().join("\n");
    t.brief = Some(brief);
    t.goal_seconds = def.map(|d| d.minutes as f32 * 60.0).unwrap_or(900.0);
    t.hints = vec![
        ts("Разбей задачу: что именно нужно сдать? Ответы — в жёлтые ячейки.").into(),
        ts("Сначала посчитай одну строку/одного контрагента, потом протяни.").into(),
        ts("Проверь себя: сходится ли итог с суммой исходной таблицы?").into(),
    ];
    t
}

fn size(g: &Gen, base: u32) -> u32 {
    base + 4 * u32::from(g.level.saturating_sub(1))
}

pub fn sales(g: &mut Gen) -> Result<Task> {
    let n = size(g, 14);
    let managers = g.l.sample(PEOPLE, 4);
    g.setup.header(1, 0, &[ts("Дата"), ts("Менеджер"), ts("Город"), ts("Сумма")]);
    for (c, w) in [(0, 90.0), (1, 120.0), (2, 90.0), (3, 100.0)] {
        g.setup.width(c, w);
    }
    let mut totals: BTreeMap<&str, f64> = BTreeMap::new();
    for r in 2..2 + n {
        let d = date(2026, 3, 1) + g.d.int(0, 30) as f64;
        let m = managers.get(g.d.below(managers.len())).copied().unwrap_or(ts("Петров С."));
        let city = g.d.pick(CITIES);
        let v = g.d.amount(5_000, 250_000, 500);
        g.setup.input(a1(r, 0), Cell::Date(d));
        g.setup.input(a1(r, 1), Cell::Text(m.into()));
        g.setup.input(a1(r, 2), Cell::Text(city.into()));
        g.setup.input(a1(r, 3), Cell::Num(v));
        *totals.entry(m).or_default() += v;
    }
    let last = 1 + n;
    g.setup.header(1, 5, &[ts("Менеджер"), ts("План")]);
    g.setup.width(5, 120.0);
    let mut over = 0.0;
    for (i, m) in managers.iter().enumerate() {
        let r = 2 + i as u32;
        let got = totals.get(m).copied().unwrap_or(0.0);
        let plan = ((got * g.d.int(80, 120) as f64 / 100.0) / 10_000.0).round() * 10_000.0 + 10_000.0 * f64::from(u8::from(i == 0));
        g.setup.input(a1(r, 5), Cell::Text((*m).into()));
        g.setup.input(a1(r, 6), Cell::Num(plan));
        if got >= plan {
            over += 1.0;
        }
    }
    let best =
        totals.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(m, _)| (*m).to_string()).unwrap_or_default();
    for (r, label) in [(1, ts("Выручка за март:")), (2, ts("Лучший менеджер:")), (3, ts("Выполнили план, чел.:"))]
    {
        g.setup.text(a1(r, 8), label);
        g.setup.bold.push(a1(r, 8));
        g.setup.answer(a1(r, 9));
    }
    g.setup.width(8, 170.0);
    g.setup.width(9, 120.0);
    push(g, "fn.sum", ts("Выручка за март (J1)"), Kind::Formula { cell: "J1".into(), reference: format!("=SUM(D2:D{last})"), live: true });
    push(g, "fn.minmax", ts("Лучший менеджер (J2)"), Kind::Value { cell: "J2".into(), expected: Cell::Text(best) });
    push(g, "logic.if", ts("Выполнили план (J3)"), Kind::Formula { cell: "J3".into(), reference: constant(over), live: false });
    let pairs = managers.iter().map(|m| ((*m).to_string(), format!("=SUMIF(B2:B{last},\"{m}\",D2:D{last})"))).collect();
    push(g, "cond.sumif", ts("Продажи по менеджерам"), Kind::Pairs { pairs });
    let brief = Brief {
        from: ts("Ольга Сергеевна, коммерческий директор").into(),
        subject: ts("Итоги марта").into(),
        body: ts("Привет! Подготовь, пожалуйста, к планёрке итоги марта: сколько всего продали, кто лучший, и кто из менеджеров выполнил план. И табличку продаж по каждому менеджеру — хочу видеть цифры.").into(),
    };
    let deliverables = vec![
        ts("J1 — выручка за март (формулой).").into(),
        ts("J2 — фамилия лучшего менеджера по продажам.").into(),
        ts("J3 — сколько менеджеров выполнили план (план в F:G).").into(),
        ts("Сводка «менеджер → сумма продаж» где-нибудь на листе (сводная или СУММЕСЛИ).").into(),
    ];
    Ok(finish(g, "case.sales", brief, &deliverables, "J1".into()))
}

pub fn payroll(g: &mut Gen) -> Result<Task> {
    let n = size(g, 6);
    g.setup.header(1, 0, &[ts("Сотрудник"), ts("Оклад"), ts("Отработано, дн."), ts("Начислено"), ts("НДФЛ 13%"), ts("К выплате")]);
    for (c, w) in [(0, 130.0), (1, 90.0), (2, 120.0), (3, 110.0), (4, 100.0), (5, 110.0)] {
        g.setup.width(c, w);
    }
    g.setup.text("H1", ts("Рабочих дней в месяце:"));
    g.setup.width(7, 190.0);
    let days = g.d.int(19, 23) as f64;
    g.setup.input("I1", Cell::Num(days));
    let people = g.l.sample(PEOPLE, n as usize);
    let (mut sum_acc, mut sum_pay) = (0.0, 0.0);
    for i in 0..n {
        let r = 2 + i;
        let who = people.get(i as usize).map(|s| (*s).to_string()).unwrap_or_else(|| tr(&format!("Сотрудник {}", i + 1)));
        g.setup.text(a1(r, 0), who);
        let salary = g.d.amount(35_000, 150_000, 1_000);
        g.setup.input(a1(r, 1), Cell::Num(salary));
        let worked = if g.d.chance(60) { days } else { (days - g.d.int(1, 8) as f64).max(1.0) };
        g.setup.input(a1(r, 2), Cell::Num(worked));
        let accrued = (salary / days * worked * 100.0).round() / 100.0;
        sum_acc += accrued;
        sum_pay += accrued - (accrued * 0.13).round();
        for c in 3..=5 {
            g.setup.answer(a1(r, c));
        }
        push(
            g,
            "refs.abs",
            &tr(&format!("Начислено, строка {r}")),
            Kind::Formula { cell: a1(r, 3), reference: format!("=ROUND(B{r}/$I$1*C{r},2)"), live: true },
        );
        push(
            g,
            "acc.payroll",
            &tr(&format!("НДФЛ, строка {r}")),
            Kind::Formula { cell: a1(r, 4), reference: format!("=ROUND(ROUND(B{r}/$I$1*C{r},2)*0.13,0)"), live: true },
        );
        push(
            g,
            "acc.payroll",
            &tr(&format!("К выплате, строка {r}")),
            Kind::Formula { cell: a1(r, 5), reference: format!("=ROUND(B{r}/$I$1*C{r},2)-ROUND(ROUND(B{r}/$I$1*C{r},2)*0.13,0)"), live: true },
        );
    }
    let total = 2 + n;
    g.setup.text(a1(total, 0), ts("Итого"));
    g.setup.bold.push(a1(total, 0));
    for c in 3..=5 {
        g.setup.answer(a1(total, c));
    }
    push(g, "fn.sum", ts("Итого начислено"), Kind::Formula { cell: a1(total, 3), reference: constant(sum_acc), live: false });
    push(g, "fn.sum", ts("Итого к выплате"), Kind::Formula { cell: a1(total, 5), reference: constant(sum_pay), live: false });
    let brief = Brief {
        from: ts("Марина Викторовна, главный бухгалтер").into(),
        subject: ts("Ведомость за месяц").into(),
        body: ts("Добрый день! Посчитай, пожалуйста, ведомость: начисление по отработанным дням (оклад / рабочие дни × отработано, до копеек), НДФЛ 13% до рубля и к выплате. Внизу итоги. Ведомость должна пересчитываться, если поменяются дни.").into(),
    };
    let deliverables = vec![
        ts("D — начислено: оклад / рабочие дни (I1) × отработано, ОКРУГЛ до копеек.").into(),
        ts("E — НДФЛ 13% от начисленного, ОКРУГЛ до рубля.").into(),
        ts("F — к выплате = начислено − НДФЛ.").into(),
        tr(&format!("Строка {total} — итоги по столбцам D и F.")),
    ];
    Ok(finish(g, "case.payroll", brief, &deliverables, "D2".into()))
}

pub fn bank(g: &mut Gen) -> Result<Task> {
    let n = size(g, 10);
    g.setup.text("A1", ts("Журнал оплат (наш учёт)"));
    g.setup.bold.push("A1".into());
    g.setup.header(2, 0, &[ts("Дата"), ts("№ п/п"), ts("Контрагент"), ts("Сумма"), ts("В выписке?")]);
    g.setup.text("G1", ts("Выписка банка"));
    g.setup.bold.push("G1".into());
    g.setup.header(2, 6, &[ts("Дата"), ts("№ п/п"), ts("Сумма")]);
    for (c, w) in [(0, 90.0), (1, 70.0), (2, 130.0), (3, 100.0), (4, 100.0), (6, 90.0), (7, 70.0), (8, 100.0)] {
        g.setup.width(c, w);
    }
    let k = 2 + usize::from(g.level >= 3);
    let mut missing: Vec<usize> = (0..n as usize).collect();
    g.l.shuffle(&mut missing);
    missing.truncate(k);
    let mut bank = Vec::new();
    let (mut cnt, mut sum) = (0.0, 0.0);
    for i in 0..n as usize {
        let r = 3 + i as u32;
        let d = date(2026, 4, 1) + g.d.int(0, 25) as f64;
        let no = 100.0 + i as f64 * 3.0 + g.l.int(0, 2) as f64;
        let v = g.d.amount(2_000, 300_000, 50);
        g.setup.input(a1(r, 0), Cell::Date(d));
        g.setup.input(a1(r, 1), Cell::Num(no));
        g.setup.input(a1(r, 2), Cell::Text(g.d.pick(PARTIES).into()));
        g.setup.input(a1(r, 3), Cell::Num(v));
        let present = !missing.contains(&i);
        let cell = a1(r, 4);
        g.setup.answer(cell.clone());
        push(g, "cond.countif", &format!("E{r}"), Kind::Value { cell, expected: Cell::Text(if present { ts("Да") } else { ts("Нет") }.into()) });
        if present {
            bank.push((d + g.d.int(0, 1) as f64, no, v));
        } else {
            cnt += 1.0;
            sum += v;
        }
    }
    g.l.shuffle(&mut bank);
    for (j, (d, no, v)) in bank.iter().enumerate() {
        let r = 3 + j as u32;
        g.setup.input(a1(r, 6), Cell::Date(*d));
        g.setup.input(a1(r, 7), Cell::Num(*no));
        g.setup.input(a1(r, 8), Cell::Num(*v));
    }
    for (r, label) in [(2, ts("Не прошло платежей:")), (3, ts("На сумму:"))] {
        g.setup.text(a1(r, 10), label);
        g.setup.bold.push(a1(r, 10));
        g.setup.answer(a1(r, 11));
    }
    g.setup.width(10, 170.0);
    push(g, "cond.countif", ts("Не прошло платежей (L2)"), Kind::Formula { cell: "L2".into(), reference: constant(cnt), live: false });
    push(g, "cond.sumif", ts("На сумму (L3)"), Kind::Formula { cell: "L3".into(), reference: constant(sum), live: false });
    let brief = Brief {
        from: ts("Марина Викторовна, главный бухгалтер").into(),
        subject: ts("Сверь оплаты с выпиской").into(),
        body: ts("Привет! Поставщики жалуются, что не получили деньги. Сверь, пожалуйста, наш журнал оплат с выпиской банка по номеру платёжки: какие платежи не прошли и на какую сумму.").into(),
    };
    let deliverables = vec![
        ts("E — для каждой строки журнала «Да», если платёжка есть в выписке, иначе «Нет» (сверяй по № п/п).").into(),
        ts("L2 — сколько платежей не прошло.").into(),
        ts("L3 — на какую сумму.").into(),
    ];
    Ok(finish(g, "case.bank", brief, &deliverables, "E3".into()))
}

pub fn act(g: &mut Gen) -> Result<Task> {
    let n = size(g, 6);
    let party = g.l.pick(PARTIES);
    g.setup.text("A1", ts("Наш учёт: расчёты с покупателем (сч. 62)"));
    g.setup.bold.push("A1".into());
    g.setup.header(2, 0, &[ts("Дата"), ts("Документ"), ts("Дебет"), ts("Кредит")]);
    g.setup.text("F1", tr(&format!("Данные {party}")));
    g.setup.bold.push("F1".into());
    g.setup.header(2, 5, &[ts("Дата"), ts("Документ"), ts("Дебет"), ts("Кредит")]);
    for (c, w) in [(0, 90.0), (1, 150.0), (2, 100.0), (3, 100.0), (5, 90.0), (6, 150.0), (7, 100.0), (8, 100.0)] {
        g.setup.width(c, w);
    }
    let lost = g.l.below(n as usize);
    let (mut our_dt, mut our_kt, mut their_dt, mut their_kt) = (0.0, 0.0, 0.0, 0.0);
    let mut lost_doc = String::new();
    let mut their_row = 3;
    for i in 0..n as usize {
        let r = 3 + i as u32;
        let d = date(2026, 1, 15) + (i as f64) * 9.0 + g.d.int(0, 5) as f64;
        let ship = i % 2 == 0;
        let doc = if ship { tr(&format!("Реализация № {}", 10 + i)) } else { tr(&format!("Оплата п/п № {}", 200 + i)) };
        let v = g.d.amount(10_000, 400_000, 100);
        g.setup.input(a1(r, 0), Cell::Date(d));
        g.setup.input(a1(r, 1), Cell::Text(doc.clone()));
        if ship {
            g.setup.input(a1(r, 2), Cell::Num(v));
            our_dt += v;
        } else {
            g.setup.input(a1(r, 3), Cell::Num(v));
            our_kt += v;
        }
        if ship && i == lost - lost % 2 {
            lost_doc = doc;
            continue;
        }
        g.setup.input(a1(their_row, 5), Cell::Date(d + g.d.int(0, 2) as f64));
        g.setup.input(a1(their_row, 6), Cell::Text(doc));
        if ship {
            g.setup.input(a1(their_row, 8), Cell::Num(v));
            their_kt += v;
        } else {
            g.setup.input(a1(their_row, 7), Cell::Num(v));
            their_dt += v;
        }
        their_row += 1;
    }
    for (r, label) in [(2, ts("Сальдо по нашим данным:")), (3, ts("Сальдо по их данным:")), (4, ts("Расхождение:")), (5, ts("Нет у контрагента:"))]
    {
        g.setup.text(a1(r, 10), label);
        g.setup.bold.push(a1(r, 10));
        g.setup.answer(a1(r, 11));
    }
    g.setup.width(10, 200.0);
    g.setup.width(11, 160.0);
    let ours = our_dt - our_kt;
    let theirs = their_kt - their_dt;
    push(g, "fn.sum", ts("Наше сальдо (L2)"), Kind::Formula { cell: "L2".into(), reference: constant(ours), live: false });
    push(g, "fn.sum", ts("Их сальдо (L3)"), Kind::Formula { cell: "L3".into(), reference: constant(theirs), live: false });
    push(g, "start.sub", ts("Расхождение (L4)"), Kind::Formula { cell: "L4".into(), reference: constant((ours - theirs).abs()), live: false });
    push(g, "acc.entries", ts("Пропавший документ (L5)"), Kind::Value { cell: "L5".into(), expected: Cell::Text(lost_doc) });
    let brief = Brief {
        from: ts("Марина Викторовна, главный бухгалтер").into(),
        subject: tr(&format!("Акт сверки с {party}")),
        body: tr(&format!(
            "{party} прислали свой акт сверки, сальдо не сходится. Посчитай сальдо по нашим данным (Дебет − Кредит) и по их (у них зеркально: Кредит − Дебет), разницу и найди документ, которого у них нет."
        )),
    };
    let deliverables = vec![
        ts("L2 — сальдо по нашим данным (дебет − кредит).").into(),
        ts("L3 — сальдо по данным контрагента (кредит − дебет).").into(),
        ts("L4 — расхождение (положительным числом).").into(),
        ts("L5 — название документа, которого нет у контрагента (как в нашей таблице).").into(),
    ];
    Ok(finish(g, "case.act", brief, &deliverables, "L2".into()))
}

pub fn receivables(g: &mut Gen) -> Result<Task> {
    let n = size(g, 10);
    let report = date(2026, 5, 31);
    g.setup.header(1, 0, &[ts("Контрагент"), ts("Счёт"), ts("Дата"), ts("Отсрочка"), ts("Сумма"), ts("Оплачено"), ts("Долг"), ts("Просрочка, дн.")]);
    for (c, w) in [(0, 130.0), (1, 70.0), (2, 90.0), (3, 80.0), (4, 100.0), (5, 100.0), (6, 100.0), (7, 120.0)] {
        g.setup.width(c, w);
    }
    g.setup.text("J1", ts("Отчётная дата:"));
    g.setup.input("K1", Cell::Date(report));
    g.setup.width(9, 170.0);
    g.setup.width(10, 110.0);
    let parties = g.l.sample(PARTIES, 4);
    let mut overdue: BTreeMap<&str, f64> = BTreeMap::new();
    let mut total = 0.0;
    for i in 0..n {
        let r = 2 + i;
        let p = parties.get(g.d.below(parties.len())).copied().unwrap_or(ts("ООО Альфа"));
        let d = date(2026, 3, 1) + g.d.int(0, 80) as f64;
        let delay = [10.0, 14.0, 30.0];
        let delay = delay.get(g.d.below(3)).copied().unwrap_or(14.0);
        let v = g.d.amount(5_000, 200_000, 100);
        let paid = match g.d.below(3) {
            0 => v,
            1 => 0.0,
            _ => (v * 0.5 / 100.0).round() * 100.0,
        };
        g.setup.input(a1(r, 0), Cell::Text(p.into()));
        g.setup.input(a1(r, 1), Cell::Num((500 + i) as f64));
        g.setup.input(a1(r, 2), Cell::Date(d));
        g.setup.input(a1(r, 3), Cell::Num(delay));
        g.setup.input(a1(r, 4), Cell::Num(v));
        g.setup.input(a1(r, 5), Cell::Num(paid));
        g.setup.answer(a1(r, 6));
        g.setup.answer(a1(r, 7));
        push(g, "start.sub", &tr(&format!("Долг, строка {r}")), Kind::Formula { cell: a1(r, 6), reference: format!("=E{r}-F{r}"), live: true });
        push(
            g,
            "date.diff",
            &tr(&format!("Просрочка, строка {r}")),
            Kind::Formula { cell: a1(r, 7), reference: format!("=IF(E{r}-F{r}>0,MAX(0,$K$1-(C{r}+D{r})),0)"), live: true },
        );
        let debt = v - paid;
        if debt > 0.0 && report - (d + delay) > 0.0 {
            *overdue.entry(p).or_default() += debt;
            total += debt;
        }
    }
    g.setup.text("J3", ts("Просрочено всего:"));
    g.setup.bold.push("J3".into());
    g.setup.answer("K3");
    push(g, "cond.sumifs", ts("Просрочено всего (K3)"), Kind::Formula { cell: "K3".into(), reference: constant(total), live: false });
    let pairs = parties.iter().map(|p| ((*p).to_string(), constant(overdue.get(p).copied().unwrap_or(0.0)))).collect();
    push(g, "cond.sumifs", ts("Просрочка по контрагентам"), Kind::Pairs { pairs });
    let brief = Brief {
        from: ts("Андрей Петрович, финансовый директор").into(),
        subject: ts("Кто нам должен").into(),
        body: tr(&format!(
            "Добрый день! Нужна картина по дебиторке на {}: долг по каждому счёту, на сколько дней просрочен, и сколько всего просрочено по каждому клиенту. Завтра звоню должникам.",
            date_text(report)
        )),
    };
    let deliverables = vec![
        ts("G — долг по счёту (сумма − оплачено).").into(),
        ts("H — дни просрочки на отчётную дату K1: если долг есть и срок (дата + отсрочка) прошёл, иначе 0.").into(),
        ts("K3 — просроченный долг всего.").into(),
        ts("Сводка «контрагент → просроченный долг» где-нибудь на листе.").into(),
    ];
    Ok(finish(g, "case.ar", brief, &deliverables, "G2".into()))
}

pub fn cleanup(g: &mut Gen) -> Result<Task> {
    let n = size(g, 10);
    g.setup.header(1, 0, &[ts("Контрагент (1С)"), ts("Сумма (1С)"), "", ts("Контрагент"), ts("Сумма")]);
    for (c, w) in [(0, 190.0), (1, 110.0), (2, 30.0), (3, 150.0), (4, 110.0)] {
        g.setup.width(c, w);
    }
    let parties = g.l.sample(PARTIES, 3);
    let mut totals: BTreeMap<&str, f64> = BTreeMap::new();
    let mut all = 0.0;
    for i in 0..n {
        let r = 2 + i;
        let p = parties.get(g.d.below(parties.len())).copied().unwrap_or(ts("ООО Альфа"));
        let messy = match g.d.below(3) {
            0 => format!("  {p}"),
            1 => format!("{p}  "),
            _ => p.replace(' ', "   "),
        };
        let v = g.d.int(1_000, 600_000) as f64;
        g.setup.input(a1(r, 0), Cell::Text(messy));
        g.setup.input(a1(r, 1), Cell::Text(num_text(v)));
        g.setup.answer(a1(r, 3));
        g.setup.answer(a1(r, 4));
        push(
            g,
            "text.trim",
            &tr(&format!("Контрагент, строка {r}")),
            Kind::Formula { cell: a1(r, 3), reference: format!("=TRIM(A{r})"), live: true },
        );
        push(
            g,
            "text.number",
            &tr(&format!("Сумма, строка {r}")),
            Kind::Formula { cell: a1(r, 4), reference: format!("=VALUE(SUBSTITUTE(B{r},\" \",\"\"))"), live: true },
        );
        *totals.entry(p).or_default() += v;
        all += v;
    }
    g.setup.text("G1", ts("Итого:"));
    g.setup.bold.push("G1".into());
    g.setup.answer("H1");
    g.setup.width(6, 90.0);
    g.setup.width(7, 120.0);
    push(g, "fn.sum", ts("Итого (H1)"), Kind::Formula { cell: "H1".into(), reference: constant(all), live: false });
    let pairs = parties.iter().map(|p| ((*p).to_string(), constant(totals.get(p).copied().unwrap_or(0.0)))).collect();
    push(g, "cond.sumif", ts("Итоги по контрагентам"), Kind::Pairs { pairs });
    let brief = Brief {
        from: ts("Марина Викторовна, главный бухгалтер").into(),
        subject: ts("Выгрузка не считается").into(),
        body: ts("Выгрузила из 1С реестр, а СУММ выдаёт ноль и контрагенты дублируются в фильтре. Приведи, пожалуйста, в порядок и дай итоги по каждому контрагенту.").into(),
    };
    let deliverables = vec![
        ts("D — чистое название контрагента (без лишних пробелов).").into(),
        ts("E — сумма числом.").into(),
        ts("H1 — общий итог.").into(),
        ts("Сводка «контрагент → сумма» где-нибудь на листе.").into(),
    ];
    Ok(finish(g, "case.cleanup", brief, &deliverables, "D2".into()))
}

fn label_answers(g: &mut Gen, col: u32, labels: &[&str]) {
    for (i, label) in labels.iter().enumerate() {
        let r = 1 + i as u32;
        g.setup.text(a1(r, col), *label);
        g.setup.bold.push(a1(r, col));
        g.setup.answer(a1(r, col + 1));
    }
    g.setup.width(col, 200.0);
    g.setup.width(col + 1, 120.0);
}

pub fn purchase(g: &mut Gen) -> Result<Task> {
    let n = size(g, 6);
    let suppliers = g.l.sample(PARTIES, 3);
    let mut head = vec![ts("Товар"), ts("Кол-во")];
    head.extend(suppliers.iter().copied());
    head.extend([ts("Мин. цена"), ts("Лучший поставщик"), ts("Сумма закупки")]);
    g.setup.header(1, 0, &head);
    for (c, w) in [(0, 120.0), (1, 70.0), (2, 110.0), (3, 110.0), (4, 110.0), (5, 100.0), (6, 150.0), (7, 120.0)] {
        g.setup.width(c, w);
    }
    let items = g.l.sample(PRODUCTS, n as usize);
    let (mut best_total, mut current_total) = (String::new(), String::new());
    for i in 0..n {
        let r = 2 + i;
        g.setup.text(a1(r, 0), items.get(i as usize).copied().unwrap_or(ts("Товар")));
        g.setup.input(a1(r, 1), Cell::Num(g.d.int(5, 120) as f64));
        // Three distinct prices so the cheapest supplier is unique.
        let base = g.d.int(40, 900);
        let mut offs = [0i64, 1, 2];
        g.d.shuffle(&mut offs);
        for (k, o) in offs.iter().enumerate() {
            let price = (base + o * g.d.int(3, 40)) as f64 + 0.5 * k as f64 * f64::from(u8::from(*o == 0));
            g.setup.input(a1(r, 2 + k as u32), Cell::Num(price));
        }
        for c in 5..=7 {
            g.setup.answer(a1(r, c));
        }
        push(
            g,
            "fn.minmax",
            &tr(&format!("Мин. цена, строка {r}")),
            Kind::Formula { cell: a1(r, 5), reference: format!("=MIN(C{r}:E{r})"), live: true },
        );
        push(
            g,
            "lookup.index",
            &tr(&format!("Поставщик, строка {r}")),
            Kind::Formula { cell: a1(r, 6), reference: format!("=INDEX($C$1:$E$1,MATCH(MIN(C{r}:E{r}),C{r}:E{r},0))"), live: true },
        );
        push(
            g,
            "refs.fill",
            &tr(&format!("Сумма, строка {r}")),
            Kind::Formula { cell: a1(r, 7), reference: format!("=B{r}*MIN(C{r}:E{r})"), live: true },
        );
        best_total.push_str(&format!("+B{r}*MIN(C{r}:E{r})"));
        current_total.push_str(&format!("+B{r}*C{r}"));
    }
    label_answers(g, 9, &[ts("Итого по лучшим ценам:"), ts("Экономия против первого поставщика:")]);
    push(
        g,
        "fn.sum",
        ts("Итого (K1)"),
        Kind::Formula { cell: "K1".into(), reference: format!("={}", best_total.trim_start_matches('+')), live: true },
    );
    push(
        g,
        "fn.sum",
        ts("Экономия (K2)"),
        Kind::Formula {
            cell: "K2".into(),
            reference: format!("=({})-({})", current_total.trim_start_matches('+'), best_total.trim_start_matches('+')),
            live: true,
        },
    );
    let first = suppliers.first().copied().unwrap_or(ts("первый поставщик"));
    let brief = Brief {
        from: ts("Айдар Маратович, руководитель закупок").into(),
        subject: ts("Коммерческие предложения").into(),
        body: tr(&format!(
            "Пришли КП от трёх поставщиков. Для каждой позиции найди минимальную цену и кто её дал, посчитай сумму закупки. И сколько сэкономим, если уйдём от «{first}», у которого сейчас всё берём. Цены будут обновляться — нужны формулы."
        )),
    };
    let deliverables = vec![
        ts("F — минимальная цена по позиции.").into(),
        ts("G — название поставщика с этой ценой (ИНДЕКС+ПОИСКПОЗ).").into(),
        ts("H — сумма закупки = кол-во × мин. цена.").into(),
        ts("K1 — итого по лучшим ценам; K2 — экономия против первого поставщика (столбец C).").into(),
    ];
    Ok(finish(g, "case.purchase", brief, &deliverables, "F2".into()))
}

pub fn stock(g: &mut Gen) -> Result<Task> {
    let n = size(g, 7);
    g.setup.header(
        1,
        0,
        &[ts("Товар"), ts("На начало"), ts("Приход"), ts("Расход"), ts("Цена"), ts("Мин. запас"), ts("На конец"), ts("Заказать?"), ts("Стоимость")],
    );
    for (c, w) in [(0, 120.0), (1, 85.0), (2, 75.0), (3, 75.0), (4, 70.0), (5, 90.0), (6, 85.0), (7, 90.0), (8, 100.0)] {
        g.setup.width(c, w);
    }
    let items = g.l.sample(PRODUCTS, n as usize);
    let (mut value, mut to_order) = (String::new(), 0.0);
    for i in 0..n {
        let r = 2 + i;
        g.setup.text(a1(r, 0), items.get(i as usize).copied().unwrap_or(ts("Товар")));
        let start = g.d.int(10, 200);
        let inc = g.d.int(0, 150);
        let out = g.d.int(0, start + inc);
        let min = g.d.int(10, 60);
        g.setup.input(a1(r, 1), Cell::Num(start as f64));
        g.setup.input(a1(r, 2), Cell::Num(inc as f64));
        g.setup.input(a1(r, 3), Cell::Num(out as f64));
        g.setup.input(a1(r, 4), Cell::Num(g.d.int(30, 2500) as f64));
        g.setup.input(a1(r, 5), Cell::Num(min as f64));
        if start + inc - out < min {
            to_order += 1.0;
        }
        for c in 6..=8 {
            g.setup.answer(a1(r, c));
        }
        push(
            g,
            "start.sub",
            &tr(&format!("На конец, строка {r}")),
            Kind::Formula { cell: a1(r, 6), reference: format!("=B{r}+C{r}-D{r}"), live: true },
        );
        push(
            g,
            "logic.if",
            &tr(&format!("Заказать, строка {r}")),
            Kind::Formula { cell: a1(r, 7), reference: tr(&format!("=IF(B{r}+C{r}-D{r}<F{r},\"Да\",\"Нет\")")), live: true },
        );
        push(
            g,
            "start.mul",
            &tr(&format!("Стоимость, строка {r}")),
            Kind::Formula { cell: a1(r, 8), reference: format!("=(B{r}+C{r}-D{r})*E{r}"), live: true },
        );
        value.push_str(&format!("+(B{r}+C{r}-D{r})*E{r}"));
    }
    label_answers(g, 10, &[ts("Стоимость остатков:"), ts("Позиций к заказу:")]);
    push(
        g,
        "fn.sum",
        ts("Стоимость остатков (L1)"),
        Kind::Formula { cell: "L1".into(), reference: format!("={}", value.trim_start_matches('+')), live: true },
    );
    push(g, "cond.countif", ts("Позиций к заказу (L2)"), Kind::Formula { cell: "L2".into(), reference: constant(to_order), live: false });
    let brief = Brief {
        from: ts("Сергей, начальник склада").into(),
        subject: ts("Остатки на конец месяца").into(),
        body: ts("Посчитай остатки на конец, отметь, что пора дозаказать (если остаток ниже минимального запаса — «Да», иначе «Нет»), и сколько денег лежит на складе.").into(),
    };
    let deliverables = vec![
        ts("G — остаток на конец = начало + приход − расход.").into(),
        ts("H — «Да», если остаток ниже мин. запаса, иначе «Нет».").into(),
        ts("I — стоимость остатка = остаток × цена.").into(),
        ts("L1 — общая стоимость остатков; L2 — сколько позиций заказать.").into(),
    ];
    Ok(finish(g, "case.stock", brief, &deliverables, "G2".into()))
}

pub fn budget(g: &mut Gen) -> Result<Task> {
    let lines = [
        ts("Аренда"),
        ts("Зарплата"),
        ts("Реклама"),
        ts("Связь"),
        ts("Канцтовары"),
        ts("Транспорт"),
        ts("Обучение"),
        ts("Командировки"),
        ts("IT-сервисы"),
        ts("Представительские"),
    ];
    let n = size(g, 6).min(lines.len() as u32);
    g.setup.header(1, 0, &[ts("Статья"), ts("План"), ts("Факт"), ts("Отклонение"), ts("Исполнение, %"), ts("Статус")]);
    for (c, w) in [(0, 150.0), (1, 100.0), (2, 100.0), (3, 110.0), (4, 120.0), (5, 120.0)] {
        g.setup.width(c, w);
    }
    for i in 0..n {
        let r = 2 + i;
        g.setup.text(a1(r, 0), lines.get(i as usize).copied().unwrap_or(ts("Статья")));
        let plan = g.d.amount(20_000, 900_000, 1_000);
        let fact = (plan * g.d.int(70, 130) as f64 / 100.0).round();
        g.setup.input(a1(r, 1), Cell::Num(plan));
        g.setup.input(a1(r, 2), Cell::Num(fact));
        for c in 3..=5 {
            g.setup.answer(a1(r, c));
        }
        push(g, "start.sub", &tr(&format!("Отклонение, строка {r}")), Kind::Formula { cell: a1(r, 3), reference: format!("=C{r}-B{r}"), live: true });
        push(g, "start.div", &tr(&format!("Исполнение, строка {r}")), Kind::Formula { cell: a1(r, 4), reference: format!("=C{r}/B{r}"), live: true });
        push(
            g,
            "logic.if",
            &tr(&format!("Статус, строка {r}")),
            Kind::Formula { cell: a1(r, 5), reference: tr(&format!("=IF(C{r}>B{r}*1.1,\"Перерасход\",\"В норме\")")), live: true },
        );
    }
    let last = 1 + n;
    let total = last + 1;
    g.setup.text(a1(total, 0), ts("Итого"));
    g.setup.bold.push(a1(total, 0));
    for c in 1..=3 {
        g.setup.answer(a1(total, c));
    }
    push(g, "fn.sum", ts("Итого план"), Kind::Formula { cell: a1(total, 1), reference: format!("=SUM(B2:B{last})"), live: true });
    push(g, "fn.sum", ts("Итого факт"), Kind::Formula { cell: a1(total, 2), reference: format!("=SUM(C2:C{last})"), live: true });
    push(
        g,
        "fn.sum",
        ts("Итого отклонение"),
        Kind::Formula { cell: a1(total, 3), reference: format!("=SUM(C2:C{last})-SUM(B2:B{last})"), live: true },
    );
    let brief = Brief {
        from: ts("Елена Петровна, финансовый директор").into(),
        subject: ts("План-факт за квартал").into(),
        body: ts("Нужен план-факт по статьям: отклонение в рублях, исполнение в процентах и статус — «Перерасход», если факт больше плана больше чем на 10%, иначе «В норме». Внизу — итоги.").into(),
    };
    let deliverables = vec![
        ts("D — отклонение = факт − план.").into(),
        ts("E — исполнение = факт / план (можно формат %).").into(),
        ts("F — «Перерасход», если факт > план × 1,1, иначе «В норме».").into(),
        tr(&format!("Строка {total} — итоги по B, C, D.")),
    ];
    Ok(finish(g, "case.budget", brief, &deliverables, "D2".into()))
}

pub fn vacation(g: &mut Gen) -> Result<Task> {
    let n = size(g, 8);
    g.setup.header(1, 0, &[ts("Сотрудник"), ts("Отдел"), ts("Начало"), ts("Окончание"), ts("Дней")]);
    for (c, w) in [(0, 130.0), (1, 110.0), (2, 100.0), (3, 100.0), (4, 70.0)] {
        g.setup.width(c, w);
    }
    let people = g.l.sample(PEOPLE, PEOPLE.len());
    let depts = g.l.sample(DEPTS, 3);
    let mut longest = (0i64, String::new());
    let mut total = 0.0;
    for i in 0..n {
        let r = 2 + i;
        let who = people.get(i as usize).map(|s| (*s).to_string()).unwrap_or_else(|| tr(&format!("Сотрудник {}", i + 1)));
        g.setup.text(a1(r, 0), who.clone());
        let dept = depts.get(g.d.below(depts.len())).copied().unwrap_or(ts("Склад"));
        g.setup.input(a1(r, 1), Cell::Text(dept.into()));
        let start = date(2026, 6, 1) + g.d.int(0, 80) as f64;
        // Distinct lengths so the longest vacation is unique.
        let days = 5 + i as i64 * 2 + g.d.int(0, 1);
        g.setup.input(a1(r, 2), Cell::Date(start));
        g.setup.input(a1(r, 3), Cell::Date(start + days as f64 - 1.0));
        g.setup.answer(a1(r, 4));
        push(g, "date.diff", &tr(&format!("Дней, строка {r}")), Kind::Formula { cell: a1(r, 4), reference: format!("=D{r}-C{r}+1"), live: true });
        total += days as f64;
        if days > longest.0 {
            longest = (days, who);
        }
    }
    let last = 1 + n;
    label_answers(g, 6, &[ts("Всего дней отпуска:"), ts("Самый длинный отпуск у:")]);
    push(
        g,
        "fn.sum",
        ts("Всего дней (H1)"),
        Kind::Formula { cell: "H1".into(), reference: format!("=SUM(D2:D{last})-SUM(C2:C{last})+{n}"), live: true },
    );
    push(g, "fn.minmax", ts("Самый длинный (H2)"), Kind::Value { cell: "H2".into(), expected: Cell::Text(longest.1) });
    let pairs = depts
        .iter()
        .map(|d| {
            ((*d).to_string(), format!("=SUMIF(B2:B{last},\"{d}\",D2:D{last})-SUMIF(B2:B{last},\"{d}\",C2:C{last})+COUNTIF(B2:B{last},\"{d}\")"))
        })
        .collect();
    push(g, "cond.sumif", ts("Дни по отделам"), Kind::Pairs { pairs });
    let _ = total;
    let brief = Brief {
        from: ts("Анна, HR-менеджер").into(),
        subject: ts("График отпусков на лето").into(),
        body: ts("Посчитай, пожалуйста, сколько календарных дней отпуска у каждого (оба дня включительно), сколько всего, у кого самый длинный, и сколько дней отпуска приходится на каждый отдел.").into(),
    };
    let deliverables = vec![
        ts("E — дней отпуска (окончание − начало + 1).").into(),
        ts("H1 — всего дней; H2 — фамилия с самым длинным отпуском.").into(),
        ts("Сводка «отдел → дней» где-нибудь на листе.").into(),
    ];
    Ok(finish(g, "case.vacation", brief, &deliverables, "E2".into()))
}

pub fn retail(g: &mut Gen) -> Result<Task> {
    let n = size(g, 12);
    let stores = [ts("Пятёрочка на Ленина"), ts("Магазин у дома №3"), ts("Супермаркет Центр"), ts("Дискаунтер Северный")];
    let cats = [ts("Молочка"), ts("Овощи"), ts("Бакалея"), ts("Напитки")];
    g.setup.header(1, 0, &[ts("Магазин"), ts("Категория"), ts("Выручка"), ts("Чеков"), ts("Средний чек")]);
    for (c, w) in [(0, 180.0), (1, 100.0), (2, 100.0), (3, 70.0), (4, 110.0)] {
        g.setup.width(c, w);
    }
    let mut by_store: BTreeMap<&str, f64> = BTreeMap::new();
    let mut all = 0.0;
    for i in 0..n {
        let r = 2 + i;
        // Every store appears; the order is shuffled by the data seed.
        let store = if (i as usize) < stores.len() { stores.get(i as usize).copied().unwrap_or(ts("Магазин")) } else { g.d.pick(&stores) };
        let cat = g.d.pick(&cats);
        let rev = g.d.amount(40_000, 900_000, 10) + i as f64;
        let checks = g.d.int(80, 1500) as f64;
        g.setup.input(a1(r, 0), Cell::Text(store.into()));
        g.setup.input(a1(r, 1), Cell::Text(cat.into()));
        g.setup.input(a1(r, 2), Cell::Num(rev));
        g.setup.input(a1(r, 3), Cell::Num(checks));
        g.setup.answer(a1(r, 4));
        push(
            g,
            "fn.round",
            &tr(&format!("Средний чек, строка {r}")),
            Kind::Formula { cell: a1(r, 4), reference: format!("=ROUND(C{r}/D{r},2)"), live: true },
        );
        *by_store.entry(store).or_default() += rev;
        all += rev;
    }
    let last = 1 + n;
    let (top, top_rev) = by_store
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(s, v)| ((*s).to_string(), *v))
        .unwrap_or_default();
    label_answers(g, 6, &[ts("Лучший магазин:"), ts("Его доля выручки:")]);
    push(g, "fn.minmax", ts("Лучший магазин (H1)"), Kind::Value { cell: "H1".into(), expected: Cell::Text(top) });
    push(g, "start.div", ts("Доля (H2)"), Kind::Formula { cell: "H2".into(), reference: constant(top_rev / all), live: false });
    let pairs = stores.iter().map(|s| ((*s).to_string(), format!("=SUMIF(A2:A{last},\"{s}\",C2:C{last})"))).collect();
    push(g, "cond.sumif", ts("Выручка по магазинам"), Kind::Pairs { pairs });
    let brief = Brief {
        from: ts("Дмитрий, руководитель отдела аналитики").into(),
        subject: ts("Срез по магазинам за неделю").into(),
        body: ts("Это тестовое задание. Посчитай средний чек по каждой строке (до копеек), выручку по каждому магазину, какой магазин лучший и какую долю общей выручки он даёт.").into(),
    };
    let deliverables = vec![
        ts("E — средний чек = выручка / чеки, ОКРУГЛ до копеек.").into(),
        ts("Сводка «магазин → выручка» где-нибудь на листе (сводная или СУММЕСЛИ).").into(),
        ts("H1 — лучший магазин; H2 — его доля в общей выручке (число или %).").into(),
    ];
    Ok(finish(g, "case.retail", brief, &deliverables, "E2".into()))
}

/// Two messy exports of the same invoices: ours from the accounting system, the supplier's from
/// their e-mail. Different number formats, text amounts and dates, extra columns, an empty row,
/// a total row, a doubled line. Graded by values; any method and either sign convention.
pub fn recon(g: &mut Gen) -> Result<Task> {
    let n = size(g, 9) as usize;
    let party = g.l.pick(PARTIES);
    let messy = g.level >= 3;
    // Distinct document numbers in date order.
    let mut docs: Vec<(u32, f64, f64)> = Vec::new();
    let mut no = 100 + g.l.int(0, 40) as u32;
    let mut day = date(2026, 3, 2);
    for _ in 0..n {
        no += 1 + g.l.int(0, 6) as u32;
        day += g.l.int(1, 3) as f64;
        docs.push((no, day, g.d.amount(3_000, 250_000, 10) + g.d.int(0, 99) as f64 / 100.0));
    }
    // Discrepancies: positions are layout, amounts are data.
    let mut idx: Vec<usize> = (0..n).collect();
    g.l.shuffle(&mut idx);
    let only_ours = idx.first().copied().unwrap_or(0);
    let only_theirs = idx.get(1).copied().unwrap_or(1);
    let typo = idx.get(2).copied().unwrap_or(2);
    let doubled = if g.level >= 2 { idx.get(3).copied() } else { None };
    let typo2 = if messy { idx.get(4).copied() } else { None };
    let mut findings: Vec<(u32, f64)> = Vec::new();
    let mut ours: Vec<(u32, f64, f64)> = Vec::new();
    let mut theirs: Vec<(u32, f64, f64)> = Vec::new();
    for (i, &(no, day, v)) in docs.iter().enumerate() {
        if i != only_theirs {
            ours.push((no, day, v));
        }
        if Some(i) == doubled {
            ours.push((no, day, v));
            findings.push((no, v));
        }
        if i == only_ours {
            findings.push((no, v));
            continue;
        }
        if i == only_theirs {
            theirs.push((no, day, v));
            findings.push((no, -v));
            continue;
        }
        if i == typo || Some(i) == typo2 {
            // A swapped pair of digits or a lost hundred.
            let wrong = if g.d.chance(50) { v + 900.0 } else { (v - 100.0).max(10.0) };
            theirs.push((no, day + 1.0, wrong));
            findings.push((no, v - wrong));
            continue;
        }
        theirs.push((no, day + g.d.int(0, 1) as f64, v));
    }
    findings.sort_by_key(|f| f.0);
    let net = ours.iter().map(|d| d.2).sum::<f64>() - theirs.iter().map(|d| d.2).sum::<f64>();

    g.setup.text("A1", tr(&format!("Наша выгрузка: расчёты с {party}")));
    g.setup.bold.push("A1".into());
    g.setup.header(2, 0, &[ts("Дата"), ts("Документ"), ts("Менеджер"), ts("Сумма"), ts("Комментарий")]);
    let managers = g.l.sample(PEOPLE, 3);
    for (j, (no, day, v)) in ours.iter().enumerate() {
        let r = 3 + j as u32;
        g.setup.input(a1(r, 0), Cell::Date(*day));
        let doc = if g.level == 1 { Cell::Num(f64::from(*no)) } else { Cell::Text(tr(&format!("СФ-{no:05}"))) };
        g.setup.input(a1(r, 1), doc);
        g.setup.input(a1(r, 2), Cell::Text(managers.get(j % managers.len().max(1)).copied().unwrap_or("").into()));
        g.setup.input(a1(r, 3), Cell::Num(*v));
        if j % 4 == 1 {
            g.setup.input(a1(r, 4), Cell::Text(ts("отгрузка по договору").into()));
        }
    }
    g.setup.text("G1", tr(&format!("Акт от {party} (из письма)")));
    g.setup.bold.push("G1".into());
    g.setup.header(2, 6, &[ts("№ счёта"), ts("Дата"), ts("Сумма с НДС"), ts("Статус")]);
    let gap = if messy { Some(theirs.len() / 2) } else { None };
    let mut r = 3;
    let mut total = 0.0;
    for (j, (no, day, v)) in theirs.iter().enumerate() {
        if Some(j) == gap {
            r += 1;
        }
        let doc = if messy { Cell::Text(format!("№ {no} ")) } else { Cell::Num(f64::from(*no)) };
        g.setup.input(a1(r, 6), doc);
        let when = if messy { Cell::Text(date_text(*day)) } else { Cell::Date(*day) };
        g.setup.input(a1(r, 7), when);
        let amount = if g.level >= 2 { Cell::Text(num_text(*v)) } else { Cell::Num(*v) };
        g.setup.input(a1(r, 8), amount);
        g.setup.input(a1(r, 9), Cell::Text(if j % 3 == 0 { ts("оплачен") } else { ts("ожидает оплаты") }.into()));
        total += v;
        r += 1;
    }
    g.setup.input(a1(r, 6), Cell::Text(ts("Итого").into()));
    g.setup.input(a1(r, 8), Cell::Num((total * 100.0).round() / 100.0));
    for (c, w) in [(0, 90.0), (1, 100.0), (2, 110.0), (3, 100.0), (4, 150.0), (6, 90.0), (7, 90.0), (8, 110.0), (9, 120.0)] {
        g.setup.width(c, w);
    }
    for (row, label) in [(2, ts("Документов с расхождениями:")), (3, ts("Чистое расхождение, ₽:"))] {
        g.setup.text(a1(row, 11), label);
        g.setup.bold.push(a1(row, 11));
        g.setup.answer(a1(row, 12));
    }
    g.setup.width(11, 200.0);
    let count = findings.len() as f64;
    push(g, "cond.countif", ts("Документов с расхождениями (M2)"), Kind::Amount { cell: "M2".into(), expected: count, either_sign: false });
    push(
        g,
        "cond.sumif",
        ts("Чистое расхождение (M3)"),
        Kind::Amount { cell: "M3".into(), expected: (net * 100.0).round() / 100.0, either_sign: true },
    );
    push(g, "lookup.xlookup", ts("Список расхождений"), Kind::Findings { items: findings });
    let brief = Brief {
        from: ts("Марина Викторовна, главный бухгалтер").into(),
        subject: tr(&format!("Не сходимся с {party}")),
        body: tr(&format!(
            "Добрый день! {party} прислали свой акт, и сальдо не сходится с нашим. Найди, пожалуйста, до конца дня, по каким документам расхождения и на сколько. Учти: их выгрузка из почты, там всё текстом и с мусором, а у нас бывают задвоения. Как делать — решай сам."
        )),
    };
    let deliverables = vec![
        ts("Список расхождений где угодно на листе: номер документа и разница рядом (знак — как тебе удобнее).").into(),
        ts("M2 — сколько документов с расхождениями (задвоение — тоже расхождение).").into(),
        ts("M3 — чистое расхождение: наша сумма минус их сумма (без строки «Итого»).").into(),
    ];
    Ok(finish(g, "case.recon", brief, &deliverables, "L2".into()))
}
